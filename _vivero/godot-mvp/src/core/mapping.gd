class_name Mapping
extends RefCounted

## El mapeo espacio -> música. La geometría vive en `Layouts`, el catálogo en
## `Biomes`; aquí queda lo puramente musical, que es lo que NO cambia entre
## ambientes:
##
##   paso (ángulo, X…)  -> posición en el ciclo
##   nivel (anillo, Y…) -> grado de la escala
##   tipo de objeto     -> timbre + octava base (la masa visual da el registro)
##   densidad           -> capas globales y apertura del filtro maestro
##   momento del día    -> escala, tónica, tempo, luz
##
## Que un acuario y un bosque compartan este archivo es justo lo que hace que
## añadir un ambiente sea trabajo de arte y no de ingeniería.

## Subdivisión por defecto. La real la fija el bioma (`Biomes.MUSIC`).
const STEP_BEATS := 0.5

# ---- variación ----
#
# El problema medido: la pieza se repetía LITERALMENTE cada ciclo (pico de
# autocorrelación en 1.00 ciclos). Cualquier bucle de 7 s sin evolución cansa en
# menos de un minuto, da igual el material.
#
# Tres capas de variación, todas DETERMINISTAS: el mismo jardín suena siempre
# igual —el render offline y la sesión en vivo coinciden— pero tarda mucho en
# repetirse.
#
#   1. Periodo por celda: no todo suena en cada vuelta. Periodos coprimos
#      (2,3,5) hacen que el conjunto solo se repita cada 30 ciclos.
#   2. Arco de densidad: la textura respira a lo largo de 24 ciclos.
#   3. Deriva de intensidad: cada celda tiene su propio ciclo lento de dinámica.
#
# Combinados, el patrón completo tarda mcm(30, 24) = 120 ciclos en repetirse:
# de 7 segundos a 14 minutos en el bosque, y a más de una hora en el cielo.

const PERIODS_LONG := [2, 3, 5]     ## sonidos largos: espacian
const PERIODS_MID := [1, 2, 3]
const PERIODS_SHORT := [1, 2]       ## percusión y notas cortas: sostienen el pulso
## Duración objetivo del arco de densidad y de la deriva, EN SEGUNDOS.
##
## Estaban en ciclos, y con ciclos de 32 s el arco del cielo tardaba 13 minutos
## en completarse: cualquier render de dos minutos caía entero en su tramo más
## callado y sonaba vacío. Igual que el intervalo de repetición, lo que tiene que
## ser constante es el tiempo.
const ARC_SECONDS := 105.0
const DRIFT_SECONDS := 74.0         ## no múltiplo de ARC_SECONDS, a propósito

## Probabilidad base de disparo según el carácter del bioma.
## Medido: con 0.50, el cielo daba 8 notas en dos minutos — eso no es esparso,
## es silencio. Los ciclos largos ya espacian por sí solos.
const FEEL_PROB := {
	"pulsed": 0.95, "natural": 0.82, "floating": 0.78, "sparse": 0.70,
}

# Umbrales de las capas, como FRACCIÓN de la rejilla.
#
# Eran números fijos (6/14/24) y eso fallaba de dos maneras: una rejilla de 60
# celdas y otra de 100 llegaban a la misma cuenta con densidades muy distintas,
# y sobre todo el AIRE (umbral 24) casi nunca entraba — siendo la capa que MÁS
# distingue un ambiente de otro (2 kHz en el agua contra 9.7 kHz en el cielo).
#
# El orden también estaba al revés: el aire es ambiente y debe estar casi
# siempre; el pulso es lo más eventual y debe ser lo último en llegar.
const LAYER_DRONE_FRAC := 0.05
const LAYER_AIR_FRAC := 0.09
const LAYER_PULSE_FRAC := 0.28
const LAYER_DRONE_MIN := 4
const LAYER_AIR_MIN := 6
const LAYER_PULSE_MIN := 12


static func layer_thresholds(total_cells: int) -> Dictionary:
	return {
		"drone": maxi(LAYER_DRONE_MIN, int(round(LAYER_DRONE_FRAC * total_cells))),
		"aire": maxi(LAYER_AIR_MIN, int(round(LAYER_AIR_FRAC * total_cells))),
		"pulso": maxi(LAYER_PULSE_MIN, int(round(LAYER_PULSE_FRAC * total_cells))),
	}

const NOTE_NAMES := ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]

## Rango de transposición manual, en semitonos. Es una decisión de composición
## (se guarda con el jardín), no un control de cámara.
const TRANSPOSE_MIN := -12
const TRANSPOSE_MAX := 12

# Reexportes, para que el resto del código no tenga que saber de `Biomes`.
const MOODS := Biomes.MOODS
const MOOD_ORDER := Biomes.MOOD_ORDER
const TYPES := Biomes.OBJECTS


# ---- gramática del bioma ----

static func step_beats(biome_id: String) -> float:
	return float(Biomes.music(biome_id)["step_beats"])


static func bpm_for(biome_id: String, mood_key: String) -> float:
	return (float(MOODS[mood_key]["bpm"])
			* float(Biomes.music(biome_id)["bpm_scale"]))


static func cycle_beats(biome_id: String, steps: int) -> float:
	return step_beats(biome_id) * float(steps)


static func degrees_of(biome_id: String, mood_key: String) -> Array:
	var own: Array = Biomes.music(biome_id)["scale"]
	return own if not own.is_empty() else MOODS[mood_key]["deg"]


static func root_of(biome_id: String, mood_key: String, transpose: int) -> int:
	return (int(MOODS[mood_key]["root"])
			+ int(Biomes.music(biome_id)["root_shift"]) + transpose)


# ---- variación ----

## FNV-1a truncado a 31 bits. Determinista y trivial de replicar en Python, que
## es lo que necesita el renderer offline para sonar igual que el juego.
static func hash_of(values: Array) -> int:
	var h := 2166136261
	for v in values:
		h = ((h ^ (int(v) & 0xffffffff)) * 16777619) & 0x7fffffff
	return h


## Intervalo objetivo entre repeticiones de una misma celda, en segundos.
## Lo que tiene que ser musicalmente constante es el TIEMPO, no el número de
## ciclos: con ciclos de 32 s, un periodo de 5 dejaba a un objeto callado 160
## segundos y el cielo daba 4 notas por minuto — silencio, no ambient.
const TARGET_REPEAT_SEC := 35.0


static func trigger_period(type_id: String, level: int, step: int,
		cycle_seconds := 7.0) -> int:
	var dur := int(TYPES[type_id]["dur"])
	var pool: Array = PERIODS_SHORT
	if dur >= 8:
		pool = PERIODS_LONG
	elif dur >= 4:
		pool = PERIODS_MID
	var cap := clampi(int(round(TARGET_REPEAT_SEC / maxf(0.5, cycle_seconds))), 1, 5)
	var allowed: Array = []
	for p in pool:
		if int(p) <= cap:
			allowed.append(p)
	if allowed.is_empty():
		allowed = [cap]
	return int(allowed[hash_of([level, step, dur]) % allowed.size()])


static func trigger_phase(level: int, step: int, period: int) -> int:
	return hash_of([step, level, 7]) % period


## 0.55..1.0 — la textura se abre y se cierra a lo largo de ARC_SECONDS.
static func density_arc(cycle: int, cycle_seconds := 7.0) -> float:
	var n := maxf(3.0, ARC_SECONDS / maxf(0.5, cycle_seconds))
	return 0.55 + 0.45 * (0.5 - 0.5 * cos(TAU * float(cycle) / n))


static func should_trigger(biome_id: String, type_id: String, level: int,
		step: int, cycle: int, cycle_seconds := 7.0) -> bool:
	var period := trigger_period(type_id, level, step, cycle_seconds)
	if posmod(cycle, period) != trigger_phase(level, step, period):
		return false
	var p := float(FEEL_PROB.get(String(Biomes.music(biome_id)["feel"]), 0.8))
	p *= density_arc(cycle, cycle_seconds)
	return float(hash_of([level, step, cycle, 11]) % 1000) / 1000.0 < p


## Deriva lenta de intensidad, con fase propia por celda para que no respiren
## todas a la vez.
static func velocity_scale(level: int, step: int, cycle: int,
		cycle_seconds := 7.0) -> float:
	var n := maxf(3.0, DRIFT_SECONDS / maxf(0.5, cycle_seconds))
	var phase := float(hash_of([level, step, 3]) % 360) / 360.0 * TAU
	return 0.72 + 0.28 * (0.5 + 0.5 * sin(TAU * float(cycle) / n + phase))


## Nota MIDI de una celda, o -1 si el tipo no tiene altura.
static func midi_of(biome_id: String, type_id: String, level: int,
		mood_key: String, transpose := 0) -> int:
	var t: Dictionary = TYPES[type_id]
	if not bool(t["pitched"]):
		return -1
	# Los grados propios del objeto (el sub, restringido a tónica y quinta) mandan
	# sobre los del bioma; si no, el bioma manda sobre el momento.
	var degs: Array = t["deg"] if t.has("deg") else degrees_of(biome_id, mood_key)
	return (root_of(biome_id, mood_key, transpose) + 12 * int(t["oct"])
			+ int(degs[level % degs.size()]))


## Factor de transposición de la muestra. La percusión varía por nivel para que
## veinte piedras no suenen a la misma piedra veinte veces.
static func pitch_scale(type_id: String, midi: int, level: int) -> float:
	var t: Dictionary = TYPES[type_id]
	if not bool(t["pitched"]):
		return 0.90 + 0.07 * float(level % 3)
	return pow(2.0, float(midi - int(t["ref"])) / 12.0)


static func note_name(midi: int) -> String:
	if midi < 0:
		return "—"
	return "%s%d" % [NOTE_NAMES[midi % 12], (midi / 12) - 1]


## Velocidad MIDI derivada del nivel de mezcla del tipo, para que el .mid
## exportado tenga el mismo equilibrio que se oye en la app.
static func velocity_of(type_id: String) -> int:
	return int(clampf(112.0 + float(TYPES[type_id]["db"]) * 2.5, 40.0, 120.0))


## Variación determinista de intensidad por celda, en dB.
##
## Deliberadamente NO se humaniza el tiempo: con los disparos resueltos por
## frame (ver conductor.gd) un desfase de 6 ms no sobrevive a la cuantización de
## 16 ms y solo añadiría ruido. El micro-timing se recupera cuando el motor de
## audio pase a FMOD/Wwise, que sí programan con precisión de sample.
static func humanize_db(level: int, step: int) -> float:
	return float((level * 7 + step * 13) % 13 - 6) * 0.25


static func color_of(type_id: String) -> Color:
	return Color(String(TYPES[type_id]["col"]))


static func scale_name(biome_id: String, mood_key: String, transpose: int) -> String:
	var own: Array = Biomes.music(biome_id)["scale"]
	var shift := int(Biomes.music(biome_id)["root_shift"]) + transpose
	var base := String(MOODS[mood_key]["scale_nm"])
	if not own.is_empty():
		# Escala propia del bioma: se nombra por su tónica y su número de grados.
		base = "%s · %d grados" % [
			NOTE_NAMES[posmod(root_of(biome_id, mood_key, transpose), 12)], own.size()]
		return base
	if shift == 0:
		return base
	return "%s %+d st" % [base, shift]
