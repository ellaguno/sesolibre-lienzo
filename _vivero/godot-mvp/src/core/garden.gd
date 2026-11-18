class_name Garden
extends RefCounted

## Estado de una escena.
##
## Un jardín es un DICCIONARIO DE CELDAS, no una lista de notas. La música se
## deriva de la escena en cada reproducción. Eso mantiene los guardados
## diminutos, hace el undo trivial y permite cambiar el mapeo o los samples sin
## invalidar lo que el jugador ya creó.
##
## La celda es `Vector2i(nivel, paso)` con independencia del layout: en un disco
## el nivel es el anillo, en un acuario es la altura. El estado no sabe de
## geometría y por eso el mismo jardín podría re-renderizarse con otro layout.

signal changed()
signal biome_changed()

const SAVE_VERSION := 2

var cells: Dictionary = {}          ## Vector2i(nivel, paso) -> String tipo
var biome_id := "bosque"
var mood_key := "atardecer"
var bpm: float = 68.0
var transpose := 0
var title := ""


func levels() -> int:
	return int(Biomes.biome(biome_id)["levels"])


func steps() -> int:
	return int(Biomes.biome(biome_id)["steps"])


func palette() -> Array:
	return Biomes.objects_of(biome_id)


func place(level: int, step: int, type_id: String) -> bool:
	if level < 0 or level >= levels() or step < 0 or step >= steps():
		return false
	# El tipo tiene que pertenecer al bioma: un pino en un acuario no es un
	# error de datos que valga la pena tolerar.
	if not palette().has(type_id):
		return false
	var k := Vector2i(level, step)
	if cells.has(k):
		return false
	cells[k] = type_id
	changed.emit()
	return true


func erase_at(level: int, step: int) -> bool:
	if not cells.erase(Vector2i(level, step)):
		return false
	changed.emit()
	return true


func clear() -> void:
	cells.clear()
	changed.emit()


func count() -> int:
	return cells.size()


func density() -> float:
	return float(cells.size()) / float(maxi(1, levels() * steps()))


func type_counts() -> Dictionary:
	var out := {}
	for type_id in cells.values():
		out[type_id] = int(out.get(type_id, 0)) + 1
	return out


## Celdas que suenan en un paso dado.
func at_step(step: int) -> Array[Vector2i]:
	var out: Array[Vector2i] = []
	for k in cells:
		if (k as Vector2i).y == step:
			out.append(k)
	return out


func set_mood(key: String) -> void:
	if not Biomes.MOODS.has(key) or key == mood_key:
		return
	mood_key = key
	bpm = Mapping.bpm_for(biome_id, key)
	changed.emit()


func set_transpose(semitones: int) -> void:
	var v := clampi(semitones, Mapping.TRANSPOSE_MIN, Mapping.TRANSPOSE_MAX)
	if v == transpose:
		return
	transpose = v
	changed.emit()


## Cambiar de bioma vacía la escena: los objetos no son intercambiables y
## traducir un bosque a un acuario produciría una escena que nadie compuso.
func set_biome(id: String) -> void:
	if not Biomes.BIOMES.has(id) or id == biome_id:
		return
	biome_id = id
	# El tempo es del bioma tanto como del momento: una ciudad no late al ritmo
	# de un acuario aunque los dos sean "noche".
	bpm = Mapping.bpm_for(biome_id, mood_key)
	cells.clear()
	biome_changed.emit()
	changed.emit()


## Escena inicial plausible, para que el jugador oiga algo desde el primer
## segundo en vez de enfrentarse a una rejilla vacía. Se genera a partir de la
## paleta del bioma, así que funciona para cualquiera sin plan escrito a mano.
func seed_default() -> void:
	cells.clear()
	var pal := palette()
	var n_lv := levels()
	var n_st := steps()
	# La paleta va de grave a agudo: se reparte por niveles en ese mismo orden,
	# de modo que los graves quedan abajo/dentro y los agudos arriba/fuera.
	# Proporcional a la rejilla: 16 objetos fijos dejaban el cielo (100 celdas)
	# mucho más vacío que el mar (60), y con ello sin apenas notas por minuto.
	var target := int(round(float(n_lv * n_st) * 0.22))
	for i in pal.size():
		var type_id := String(pal[i])
		var level := int(round(float(i) / maxf(1.0, float(pal.size() - 1))
				* float(n_lv - 1)))
		var stride := 4 if i % 2 == 0 else 6
		var offset := (i * 3) % stride
		for step in range(offset, n_st, stride):
			if cells.size() >= target:
				break
			# Si la celda está ocupada se busca el nivel vecino en vez de perder
			# el objeto en silencio.
			var lv := level
			for tries in n_lv:
				if not cells.has(Vector2i(lv, step)):
					break
				lv = (lv + 1) % n_lv
			cells[Vector2i(lv, step)] = type_id
	changed.emit()


# --------------------------------------------------------------------------
# persistencia
# --------------------------------------------------------------------------

func to_dict() -> Dictionary:
	var flat: Array = []
	for k in cells:
		var c := k as Vector2i
		flat.append({"nivel": c.x, "paso": c.y, "tipo": cells[k]})
	return {
		"version": SAVE_VERSION,
		"titulo": title,
		"bioma": biome_id,
		"momento": mood_key,
		"bpm": bpm,
		"transposicion": transpose,
		"celdas": flat,
	}


static func from_dict(d: Dictionary) -> Garden:
	var g := Garden.new()
	g.title = String(d.get("titulo", ""))
	g.biome_id = String(d.get("bioma", "bosque"))
	if not Biomes.BIOMES.has(g.biome_id):
		g.biome_id = "bosque"
	g.mood_key = String(d.get("momento", "atardecer"))
	if not Biomes.MOODS.has(g.mood_key):
		g.mood_key = "atardecer"
	g.bpm = float(d.get("bpm", Mapping.bpm_for(g.biome_id, g.mood_key)))
	g.transpose = clampi(int(d.get("transposicion", 0)),
			Mapping.TRANSPOSE_MIN, Mapping.TRANSPOSE_MAX)

	var pal := g.palette()
	for e in d.get("celdas", []):
		var type_id := String(e.get("tipo", ""))
		if not pal.has(type_id):
			continue    # tipo retirado o de otro bioma: se ignora
		# v1 usaba anillo/sector; v2 usa nivel/paso. Se aceptan ambos.
		var level := int(e.get("nivel", e.get("anillo", -1)))
		var step := int(e.get("paso", e.get("sector", -1)))
		if level >= 0 and level < g.levels() and step >= 0 and step < g.steps():
			g.cells[Vector2i(level, step)] = type_id
	return g


# --------------------------------------------------------------------------
# handoff a la fase 2
# --------------------------------------------------------------------------

## Notas del ciclo, ordenadas. La percusión se excluye a propósito: condicionar
## melodía con eventos sin altura degrada el resultado del modelo.
func melody() -> Array[Dictionary]:
	var out: Array[Dictionary] = []
	for k in cells:
		var c := k as Vector2i
		var type_id := String(cells[k])
		var midi := Mapping.midi_of(biome_id, type_id, c.x, mood_key, transpose)
		if midi < 0:
			continue
		out.append({
			"paso": c.y,
			"midi": midi,
			"nota": Mapping.note_name(midi),
			"pasos": int(Mapping.TYPES[type_id]["dur"]),
			"capa": type_id,
			"vel": Mapping.velocity_of(type_id),
		})
	out.sort_custom(func(a, b):
		return a["paso"] < b["paso"] if a["paso"] != b["paso"] else a["midi"] < b["midi"])
	return out


## Qué capas de fondo suenan. Los umbrales son relativos al tamaño de la rejilla:
## 16 objetos no significan lo mismo en 60 celdas que en 100.
func active_layers() -> Dictionary:
	return _active_layers()


func _active_layers() -> Dictionary:
	var th := Mapping.layer_thresholds(levels() * steps())
	var n := count()
	return {
		"drone": n >= int(th["drone"]),
		"aire": n >= int(th["aire"]),
		"pulso": n >= int(th["pulso"]),
	}


## Prompt estructurado derivado de la escena real, no escrito a mano.
func ai_payload() -> Dictionary:
	var counts := type_counts()
	var notes := melody()
	var avg := 0
	if not notes.is_empty():
		var sum := 0
		for n in notes:
			sum += int(n["midi"])
		avg = int(round(float(sum) / float(notes.size())))

	var by_count := counts.keys()
	by_count.sort_custom(func(a, b): return int(counts[a]) > int(counts[b]))
	var dominant: Array[String] = []
	for type_id in by_count.slice(0, 3):
		dominant.append(String(Mapping.TYPES[type_id]["sub"]).split(" · ")[0])

	var m: Dictionary = Biomes.MOODS[mood_key]
	var d := density()
	var texture := "extremadamente esparso, mucho silencio"
	if d >= 0.35:
		texture = "denso, tapiz continuo"
	elif d >= 0.15:
		texture = "esparso y contemplativo"

	var prompt_parts: Array[String] = [
		"Ambient generativo instrumental",
		"%s, %s" % [String(Biomes.biome(biome_id)["nm"]).to_lower(),
					String(m["nm"]).to_lower()],
		"%d BPM, 4/4" % int(round(bpm)),
		Mapping.scale_name(biome_id, mood_key, transpose),
	]
	if not dominant.is_empty():
		prompt_parts.append("capas dominantes: " + ", ".join(dominant))
	prompt_parts.append(texture)
	prompt_parts.append("reverb amplio, sin batería, loopable, sin voz")

	return {
		"version": SAVE_VERSION,
		"bioma": biome_id,
		"layout": String(Biomes.biome(biome_id)["layout"]),
		"momento": mood_key,
		"bpm": bpm,
		"compas": "4/4",
		"ciclo_compases": maxi(1, int(round(Mapping.cycle_beats(biome_id, steps()) / 4.0))),
		"pasos_por_ciclo": steps(),
		"paso_en_pulsos": Mapping.step_beats(biome_id),
		"ciclo_segundos": snappedf(Mapping.cycle_beats(biome_id, steps())
				* 60.0 / maxf(bpm, 1.0), 0.01),
		"caracter": String(Biomes.music(biome_id)["feel"]),
		"niveles": levels(),
		"transposicion": transpose,
		"tonica": Mapping.NOTE_NAMES[posmod(Mapping.root_of(biome_id, mood_key, transpose), 12)],
		"escala": Mapping.scale_name(biome_id, mood_key, transpose),
		"grados": Mapping.degrees_of(biome_id, mood_key),
		"densidad": snappedf(d, 0.001),
		"objetos_total": count(),
		"objetos": counts,
		"registro_medio_midi": avg,
		"capas_activas": _active_layers(),
		"prompt_texto": ", ".join(prompt_parts),
		"melodia": notes,
		"celdas": to_dict()["celdas"],
	}


# --------------------------------------------------------------------------
# Standard MIDI File (formato 0) — condicionamiento melódico para el modelo
# --------------------------------------------------------------------------

const PPQ := 480
const TICKS_PER_STEP := PPQ / 2      ## un paso del mapeo = una corchea


static func _var_len(value: int) -> PackedByteArray:
	## Cantidad de longitud variable: 7 bits por byte, bit alto = "sigue".
	var out := PackedByteArray([value & 0x7f])
	var n := value >> 7
	while n > 0:
		out.insert(0, (n & 0x7f) | 0x80)
		n >>= 7
	return out


func to_midi(cycles: int = 4) -> PackedByteArray:
	var notes := melody()
	var n_steps := steps()
	var events: Array[Dictionary] = []
	for c in cycles:
		for n in notes:
			var on := (c * n_steps + int(n["paso"])) * TICKS_PER_STEP
			var off := on + maxi(1, int(n["pasos"])) * TICKS_PER_STEP
			var pitch := clampi(int(n["midi"]), 0, 127)
			events.append({"t": on, "d": PackedByteArray([0x90, pitch, int(n["vel"])])})
			events.append({"t": off, "d": PackedByteArray([0x80, pitch, 0x40])})

	# Ordenar por tick y, a igual tick, note-off (0x80) antes de note-on (0x90),
	# para que una nota repetida no se corte a sí misma.
	events.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		if int(a["t"]) != int(b["t"]):
			return int(a["t"]) < int(b["t"])
		return int((a["d"] as PackedByteArray)[0]) < int((b["d"] as PackedByteArray)[0]))

	var track := PackedByteArray()
	var mpq := int(60_000_000.0 / maxf(bpm, 1.0))       # microsegundos por negra
	track.append_array(PackedByteArray([0x00, 0xFF, 0x51, 0x03,
		(mpq >> 16) & 0xff, (mpq >> 8) & 0xff, mpq & 0xff]))
	track.append_array(PackedByteArray([0x00, 0xFF, 0x58, 0x04, 4, 2, 24, 8]))  # 4/4

	var last := 0
	for e in events:
		track.append_array(_var_len(int(e["t"]) - last))
		track.append_array(e["d"] as PackedByteArray)
		last = int(e["t"])
	track.append_array(PackedByteArray([0x00, 0xFF, 0x2F, 0x00]))               # fin

	var out := PackedByteArray()
	out.append_array(PackedByteArray([0x4D, 0x54, 0x68, 0x64, 0, 0, 0, 6,
		0, 0,                                   # formato 0
		0, 1,                                   # una pista
		(PPQ >> 8) & 0xff, PPQ & 0xff]))
	var n := track.size()
	out.append_array(PackedByteArray([0x4D, 0x54, 0x72, 0x6B,
		(n >> 24) & 0xff, (n >> 16) & 0xff, (n >> 8) & 0xff, n & 0xff]))
	out.append_array(track)
	return out
