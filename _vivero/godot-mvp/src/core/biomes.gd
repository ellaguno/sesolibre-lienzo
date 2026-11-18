class_name Biomes
extends RefCounted

## Catálogo de ambientes. Todo es DATO: añadir un bioma no toca código.
##
## Un bioma declara qué layout usa, qué objetos ofrece y con qué luz y escalas
## suena. Los objetos viven en un registro global (`OBJECTS`) y cada bioma toma
## un subconjunto, así que compartirlos entre ambientes sale gratis.
##
## PENDIENTE por diseño: mar, selva e interior. La estructura ya los admite —
## son entradas de diccionario. Lo que falta es su set de mallas y de muestras,
## que es trabajo de arte y de composición, no de arquitectura.

## Registro global de objetos.
##   oct    -> octavas respecto a la tónica. LA MASA VISUAL DA EL REGISTRO.
##   dur    -> duración en pasos
##   size   -> escala visual (0..1)
##   shape  -> forma en MeshLib
##   deg    -> grados permitidos; si falta, usa los del momento del día
const OBJECTS := {
	# --- bosque ---
	"arbol": {"nm": "Árbol", "sub": "pad · grave", "sample": "pad", "ref": 51,
		"pitched": true, "oct": -1, "dur": 8, "size": 1.00, "col": "5f8f6a",
		"db": -10.0, "shape": "pine"},
	"hongo": {"nm": "Hongo", "sub": "sub · pulso", "sample": "sub", "ref": 36,
		"pitched": true, "oct": -2, "dur": 4, "size": 0.62, "col": "b5738c",
		"db": -7.0, "shape": "mushroom", "deg": [0, 4]},
	"helecho": {"nm": "Helecho", "sub": "arpa", "sample": "pluck", "ref": 63,
		"pitched": true, "oct": 0, "dur": 2, "size": 0.66, "col": "7fae86",
		"db": -6.0, "shape": "fern"},
	"arbusto": {"nm": "Arbusto", "sub": "marimba", "sample": "marimba", "ref": 63,
		"pitched": true, "oct": 0, "dur": 2, "size": 0.74, "col": "94b563",
		"db": -8.0, "shape": "bush"},
	"flor": {"nm": "Flor", "sub": "campana", "sample": "bell", "ref": 75,
		"pitched": true, "oct": 1, "dur": 4, "size": 0.48, "col": "e8a2b8",
		"db": -11.0, "shape": "flower"},
	"luciernaga": {"nm": "Luciérnaga", "sub": "shimmer", "sample": "shimmer", "ref": 87,
		"pitched": true, "oct": 2, "dur": 4, "size": 0.34, "col": "f5e6a3",
		"db": -14.0, "shape": "firefly"},
	"piedra": {"nm": "Piedra", "sub": "percusión", "sample": "perc", "ref": 0,
		"pitched": false, "oct": 0, "dur": 1, "size": 0.58, "col": "8d8d96",
		"db": -12.0, "shape": "rock"},

	# --- acuario ---
	"coral": {"nm": "Coral", "sub": "swell · grave", "sample": "swell", "ref": 51,
		"pitched": true, "oct": -1, "dur": 8, "size": 1.00, "col": "e08a6a",
		"db": -11.0, "shape": "coral"},
	"alga": {"nm": "Alga", "sub": "swell medio", "sample": "swell_hi", "ref": 63,
		"pitched": true, "oct": 0, "dur": 4, "size": 0.72, "col": "5fa07a",
		"db": -13.0, "shape": "kelp"},
	"anemona": {"nm": "Anémona", "sub": "vaso", "sample": "glass", "ref": 75,
		"pitched": true, "oct": 1, "dur": 4, "size": 0.66, "col": "c98ad6",
		"db": -10.0, "shape": "anemone"},
	"burbuja": {"nm": "Burbuja", "sub": "gota", "sample": "waterdrop", "ref": 75,
		"pitched": true, "oct": 1, "dur": 2, "size": 0.40, "col": "9fd8ff",
		"db": -9.0, "shape": "bubble"},
	"medusa": {"nm": "Medusa", "sub": "brillo", "sample": "shimmer", "ref": 87,
		"pitched": true, "oct": 2, "dur": 4, "size": 0.56, "col": "b7a6f0",
		"db": -13.0, "shape": "jelly"},
	"pez": {"nm": "Pez", "sub": "percusión", "sample": "blub", "ref": 0,
		"pitched": false, "oct": 0, "dur": 1, "size": 0.52, "col": "f0a860",
		"db": -11.0, "shape": "fish"},

	# --- ciudad ---
	"torre": {"nm": "Torre", "sub": "pad · grave", "sample": "pad", "ref": 51,
		"pitched": true, "oct": -1, "dur": 8, "size": 1.00, "col": "6f7a92",
		"db": -11.0, "shape": "tower"},
	"casa": {"nm": "Casa", "sub": "marimba", "sample": "marimba", "ref": 63,
		"pitched": true, "oct": 0, "dur": 2, "size": 0.70, "col": "9a8577",
		"db": -8.0, "shape": "house"},
	"antena": {"nm": "Antena", "sub": "metal", "sample": "metal", "ref": 75,
		"pitched": true, "oct": 1, "dur": 4, "size": 0.62, "col": "b8c0cc",
		"db": -10.0, "shape": "antenna"},
	"farola": {"nm": "Farola", "sub": "percusión", "sample": "click", "ref": 0,
		"pitched": false, "oct": 0, "dur": 1, "size": 0.46, "col": "d8c9a8",
		"db": -13.0, "shape": "lamp"},

	"roca_mar": {"nm": "Roca", "sub": "percusión", "sample": "perc", "ref": 0,
		"pitched": false, "oct": 0, "dur": 1, "size": 0.62, "col": "6f7a80",
		"db": -12.0, "shape": "rock"},

	# --- selva ---
	"ceiba": {"nm": "Ceiba", "sub": "pad · grave", "sample": "pad", "ref": 51,
		"pitched": true, "oct": -1, "dur": 8, "size": 1.00, "col": "4e7d55",
		"db": -10.0, "shape": "ceiba"},
	"bambu": {"nm": "Bambú", "sub": "flauta", "sample": "flute", "ref": 63,
		"pitched": true, "oct": 0, "dur": 4, "size": 0.78, "col": "9dbb63",
		"db": -9.0, "shape": "bamboo"},
	"liana": {"nm": "Liana", "sub": "arpa", "sample": "pluck", "ref": 63,
		"pitched": true, "oct": 0, "dur": 2, "size": 0.70, "col": "6d9c6a",
		"db": -7.0, "shape": "liana"},
	"orquidea": {"nm": "Orquídea", "sub": "campana", "sample": "bell", "ref": 75,
		"pitched": true, "oct": 1, "dur": 4, "size": 0.52, "col": "d98ac0",
		"db": -11.0, "shape": "orchid"},
	"ave": {"nm": "Ave", "sub": "trino", "sample": "chirp", "ref": 87,
		"pitched": true, "oct": 2, "dur": 2, "size": 0.44, "col": "f2b544",
		"db": -13.0, "shape": "bird"},
	"rana": {"nm": "Rana", "sub": "percusión", "sample": "tom", "ref": 0,
		"pitched": false, "oct": 0, "dur": 1, "size": 0.50, "col": "7fbf6a",
		"db": -11.0, "shape": "frog"},

	# --- mar ---
	"faro": {"nm": "Faro", "sub": "sirena · grave", "sample": "horn", "ref": 51,
		"pitched": true, "oct": -1, "dur": 8, "size": 1.00, "col": "e8e2d4",
		"db": -13.0, "shape": "lighthouse"},
	"barca": {"nm": "Barca", "sub": "marimba", "sample": "marimba", "ref": 63,
		"pitched": true, "oct": 0, "dur": 2, "size": 0.72, "col": "b0764a",
		"db": -8.0, "shape": "boat"},
	"boya": {"nm": "Boya", "sub": "campana", "sample": "bell", "ref": 75,
		"pitched": true, "oct": 1, "dur": 4, "size": 0.58, "col": "e35f4a",
		"db": -10.0, "shape": "buoy"},
	"gaviota": {"nm": "Gaviota", "sub": "trino", "sample": "chirp", "ref": 87,
		"pitched": true, "oct": 2, "dur": 2, "size": 0.42, "col": "f0f0f0",
		"db": -14.0, "shape": "gull"},
	"ola": {"nm": "Ola", "sub": "rompiente", "sample": "wash", "ref": 0,
		"pitched": false, "oct": 0, "dur": 4, "size": 0.66, "col": "8fd0e0",
		"db": -12.0, "shape": "wave"},

	# --- cielo ---
	"nube": {"nm": "Nube", "sub": "swell · grave", "sample": "swell", "ref": 51,
		"pitched": true, "oct": -1, "dur": 8, "size": 1.00, "col": "b9c4de",
		"db": -12.0, "shape": "cloud"},
	"luna": {"nm": "Luna", "sub": "swell medio", "sample": "swell_hi", "ref": 63,
		"pitched": true, "oct": 0, "dur": 8, "size": 0.86, "col": "f2ecd0",
		"db": -12.0, "shape": "moon"},
	"estrella": {"nm": "Estrella", "sub": "vaso", "sample": "glass", "ref": 75,
		"pitched": true, "oct": 1, "dur": 4, "size": 0.40, "col": "cfe6ff",
		"db": -11.0, "shape": "star"},
	"cometa": {"nm": "Cometa", "sub": "brillo", "sample": "shimmer", "ref": 87,
		"pitched": true, "oct": 2, "dur": 4, "size": 0.54, "col": "9fe8ff",
		"db": -13.0, "shape": "comet"},
	"meteoro": {"nm": "Meteoro", "sub": "percusión", "sample": "click", "ref": 0,
		"pitched": false, "oct": 0, "dur": 1, "size": 0.48, "col": "9a8b7a",
		"db": -13.0, "shape": "meteor"},

	# --- interior ---
	"lampara": {"nm": "Lámpara", "sub": "pad · grave", "sample": "pad", "ref": 51,
		"pitched": true, "oct": -1, "dur": 8, "size": 0.94, "col": "c9b48c",
		"db": -12.0, "shape": "floorlamp"},
	"maceta": {"nm": "Maceta", "sub": "eléctrico", "sample": "rhodes", "ref": 63,
		"pitched": true, "oct": 0, "dur": 4, "size": 0.74, "col": "7fae86",
		"db": -9.0, "shape": "planter"},
	"radio": {"nm": "Radio", "sub": "marimba", "sample": "marimba", "ref": 63,
		"pitched": true, "oct": 0, "dur": 2, "size": 0.66, "col": "a8613f",
		"db": -9.0, "shape": "radio"},
	"libro": {"nm": "Libro", "sub": "caja de música", "sample": "musicbox", "ref": 75,
		"pitched": true, "oct": 1, "dur": 4, "size": 0.56, "col": "9a5f6d",
		"db": -10.0, "shape": "book"},
	"vela": {"nm": "Vela", "sub": "brillo", "sample": "shimmer", "ref": 87,
		"pitched": true, "oct": 2, "dur": 4, "size": 0.44, "col": "ffd9a0",
		"db": -14.0, "shape": "candle"},
	"reloj": {"nm": "Reloj", "sub": "percusión", "sample": "tick", "ref": 0,
		"pitched": false, "oct": 0, "dur": 1, "size": 0.60, "col": "b7a98e",
		"db": -14.0, "shape": "clock"},
}

## Momentos del día. Cada uno es un género distinto, no un reskin.
const MOODS := {
	"amanecer": {"nm": "Amanecer", "root": 62, "deg": [0, 2, 4, 7, 9],
		"scale_nm": "D lidio pentatónico", "bpm": 62,
		"sky": "241a33", "amb": "5c3f57", "sun": "ffd9a0", "sun_energy": 0.75,
		"reverb": 0.52},
	"dia": {"nm": "Día", "root": 62, "deg": [0, 2, 3, 5, 7],
		"scale_nm": "D dórico", "bpm": 78,
		"sky": "102b25", "amb": "2f6b57", "sun": "eaf6cd", "sun_energy": 1.15,
		"reverb": 0.34},
	"atardecer": {"nm": "Atardecer", "root": 57, "deg": [0, 3, 5, 7, 10],
		"scale_nm": "A menor pentatónica", "bpm": 68,
		"sky": "26141d", "amb": "75383c", "sun": "ffb37a", "sun_energy": 0.85,
		"reverb": 0.46},
	"noche": {"nm": "Noche", "root": 55, "deg": [0, 2, 3, 7, 10],
		"scale_nm": "G eólico", "bpm": 52,
		"sky": "05090f", "amb": "16243f", "sun": "9fd8ff", "sun_energy": 0.35,
		"reverb": 0.62},
}

const MOOD_ORDER := ["amanecer", "dia", "atardecer", "noche"]

## GRAMÁTICA MUSICAL por bioma.
##
## Que los siete compartieran escala, tempo, longitud de ciclo, subdivisión y
## capas de fondo era la razón medible de que sonaran igual: la distancia entre
## sus perfiles espectrales bajaba a 0.06 entre ciudad e interior. Cambiar solo
## los timbres no arregla eso — hace falta que cada ambiente tenga su propia
## forma de contar el tiempo.
##
##   step_beats  subdivisión: 0.25 semicorchea, 0.5 corchea, 1.0 negra
##   bpm_scale   multiplicador sobre el tempo del momento
##   scale       grados propios; si es [] usa los del momento
##   root_shift  semitonos sobre la tónica del momento
##   feel        "pulsed" | "natural" | "floating" | "sparse" -> densidad de disparo
##
## Duración de ciclo resultante = pasos x step_beats x 60 / (bpm x bpm_scale).
## Va de 2.6 s en la ciudad a 32 s en el cielo: eso ya no se confunde.
const MUSIC := {
	"bosque":   {"step_beats": 0.5,  "bpm_scale": 1.00, "scale": [],
				 "root_shift": 0,  "feel": "natural",
				 "drone": "drone_loop", "air": "air_loop"},
	"selva":    {"step_beats": 0.5,  "bpm_scale": 1.15, "scale": [0, 2, 4, 6, 9],
				 "root_shift": 2,  "feel": "pulsed",
				 "drone": "drone_loop", "air": "air_loop"},
	"acuario":  {"step_beats": 1.0,  "bpm_scale": 0.75, "scale": [0, 3, 5, 10],
				 "root_shift": -5, "feel": "floating",
				 "drone": "drone_deep", "air": "air_water"},
	"mar":      {"step_beats": 1.0,  "bpm_scale": 0.60, "scale": [0, 5, 7],
				 "root_shift": -7, "feel": "sparse",
				 "drone": "drone_surge", "air": "air_surf"},
	# Medido: con bpm_scale 0.55 el ciclo duraba 32 s y salían 8 notas por minuto
	# — eso no es "inmenso", es silencio. 23 s ya separa de sobra del bosque.
	"cielo":    {"step_beats": 1.0,  "bpm_scale": 0.75, "scale": [0, 2, 7, 9],
				 "root_shift": 0,  "feel": "floating",
				 "drone": "drone_open", "air": "air_high"},
	"ciudad":   {"step_beats": 0.25, "bpm_scale": 1.35, "scale": [0, 3, 5, 7, 10],
				 "root_shift": 0,  "feel": "pulsed",
				 "drone": "drone_room", "air": "air_dust"},
	"interior": {"step_beats": 0.5,  "bpm_scale": 0.90, "scale": [0, 4, 5, 7, 11],
				 "root_shift": -3, "feel": "natural",
				 "drone": "drone_room", "air": "air_dust"},
}


static func music(biome_id: String) -> Dictionary:
	return MUSIC.get(biome_id, MUSIC["bosque"])


## Colores de suelo por bioma y momento: el mismo momento no tiene por qué
## teñir igual la tierra de un bosque que el fondo de un tanque.
const BIOMES := {
	"bosque": {
		"nm": "Bosque", "layout": "radial", "levels": 5, "steps": 16,
		"objects": ["arbol", "hongo", "helecho", "arbusto", "flor",
					"luciernaga", "piedra"],
		"ground": {"amanecer": "3b3048", "dia": "24463a",
				   "atardecer": "45252c", "noche": "121c2e"},
		"reverb_bonus": 0.0,
	},
	"acuario": {
		"nm": "Acuario", "layout": "wall", "levels": 5, "steps": 16,
		"objects": ["coral", "alga", "anemona", "burbuja", "medusa", "pez"],
		"ground": {"amanecer": "1d3350", "dia": "12495e",
				   "atardecer": "1b2f52", "noche": "06121f"},
		# Bajo el agua todo es más largo. La reverb extra no es adorno: es lo
		# que separa "acuario" de "bosque con peces".
		"reverb_bonus": 0.18,
		"ground_glow": 0.30,
	},
	"ciudad": {
		"nm": "Ciudad", "layout": "field", "levels": 5, "steps": 16,
		"objects": ["torre", "casa", "antena", "farola", "arbusto", "luciernaga"],
		"ground": {"amanecer": "2e2a33", "dia": "3a3a3c",
				   "atardecer": "33262a", "noche": "10131c"},
		# Superficies duras y planas: cola corta.
		"reverb_bonus": -0.14,
	},
	"selva": {
		"nm": "Selva", "layout": "radial", "levels": 5, "steps": 12,
		"objects": ["ceiba", "bambu", "liana", "orquidea", "ave", "rana"],
		"ground": {"amanecer": "2f3a2a", "dia": "1f4028",
				   "atardecer": "3a2f22", "noche": "0f1a15"},
		# Dosel cerrado: mucha absorción, cola corta pero difusa.
		"reverb_bonus": -0.06,
	},
	"mar": {
		"nm": "Mar", "layout": "field", "levels": 5, "steps": 12,
		"objects": ["faro", "barca", "boya", "gaviota", "ola", "roca_mar"],
		"ground": {"amanecer": "26405e", "dia": "1c5c72",
				   "atardecer": "2b3a58", "noche": "081422"},
		# Espacio abierto sin superficies cercanas: cola larga y lejana.
		"reverb_bonus": 0.22,
		"ground_glow": 0.34,
	},
	"cielo": {
		"nm": "Cielo", "layout": "dome", "levels": 5, "steps": 20,
		"objects": ["nube", "luna", "estrella", "cometa", "meteoro"],
		"ground": {"amanecer": "3a3050", "dia": "2b4a6b",
				   "atardecer": "3d2740", "noche": "070a16"},
		# Nada que refleje: la cola es enorme y difusa.
		"reverb_bonus": 0.30,
		"ground_glow": 0.14,
	},
	"interior": {
		"nm": "Interior", "layout": "wall", "levels": 5, "steps": 16,
		"objects": ["lampara", "maceta", "radio", "libro", "vela", "reloj"],
		"ground": {"amanecer": "463c34", "dia": "544a40",
				   "atardecer": "4a3830", "noche": "241f1e"},
		# Habitación pequeña con muebles: casi todo seco.
		"reverb_bonus": -0.20,
	},
}

const BIOME_ORDER := ["bosque", "selva", "acuario", "mar", "cielo", "interior", "ciudad"]


static func biome(id: String) -> Dictionary:
	return BIOMES.get(id, BIOMES["bosque"])


static func objects_of(biome_id: String) -> Array:
	return biome(biome_id)["objects"]


static func ground_color(biome_id: String, mood_key: String) -> Color:
	var g: Dictionary = biome(biome_id)["ground"]
	return Color(String(g.get(mood_key, "333333")))


## Brillo propio de la base, 0..1.
##
## Un fondo azul bajo el sol naranja del atardecer se vuelve marrón: físicamente
## correcto, pero destruye la lectura de "agua". Un poco de emisión hace que el
## agua y el cielo conserven su identidad bajo cualquier momento del día, que es
## lo que uno espera de ellos. Las superficies opacas (tierra, asfalto, madera)
## no lo llevan: ahí sí queremos que la luz mande.
static func ground_glow(biome_id: String) -> float:
	return float(biome(biome_id).get("ground_glow", 0.0))


static func reverb_of(biome_id: String, mood_key: String) -> float:
	return clampf(float(MOODS[mood_key]["reverb"])
			+ float(biome(biome_id)["reverb_bonus"]), 0.05, 0.95)
