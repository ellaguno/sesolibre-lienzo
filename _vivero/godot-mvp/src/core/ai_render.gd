extends Node

## Cliente del servicio de render de IA. Autoload `AIRender`.
##
## Modelo asíncrono: se encola el trabajo, se sondea el estado y al terminar se
## descarga el audio. Un render de IA tarda 30-90 s — más de lo que puede esperar
## una petición HTTP en móvil, y desde luego más de lo que puede bloquear un
## frame.
##
## El servicio es ../../ai-render/server.py. Si no está levantado, esto falla de
## forma limpia y el juego sigue sonando en directo como siempre.

signal started(job_id: String)
signal progress(estado: String, intento: int)
signal finished(stream: AudioStream, info: Dictionary)
signal failed(motivo: String)

const DEFAULT_URL := "http://127.0.0.1:8900"
const POLL_SECONDS := 2.0
const MAX_POLLS := 90                  ## ~3 min de techo

var base_url := DEFAULT_URL
var provider := ""                     ## vacío = el que tenga el servicio por defecto
var duration_s := 150.0

var _http: HTTPRequest
var _job_id := ""
var _polls := 0
var _busy := false
var _phase := ""                       ## "post" | "poll" | "audio"
var _info: Dictionary = {}


func _ready() -> void:
	_http = HTTPRequest.new()
	# Sin límite de tamaño en cuerpo: un WAV de 2 min son ~20 MiB y el valor por
	# defecto de HTTPRequest lo cortaría.
	_http.body_size_limit = -1
	_http.timeout = 30.0
	_http.request_completed.connect(_on_completed)
	add_child(_http)


func is_busy() -> bool:
	return _busy


## Encola un render del jardín dado. Devuelve false si ya hay uno en curso.
func request_render(garden: Garden, force := false) -> bool:
	if _busy:
		return false
	if garden.count() == 0:
		failed.emit("El jardín está vacío.")
		return false

	var body := {
		"payload": garden.ai_payload(),
		"midi_b64": Marshalls.raw_to_base64(garden.to_midi()),
		"duration_s": duration_s,
		"force": force,
	}
	if provider != "":
		body["provider"] = provider

	_busy = true
	_job_id = ""
	_polls = 0
	_phase = "post"
	_info = {}
	var err := _http.request(
		base_url + "/render",
		["Content-Type: application/json"],
		HTTPClient.METHOD_POST,
		JSON.stringify(body))
	if err != OK:
		_fail("No se pudo contactar el servicio (error %d). ¿Está corriendo server.py?" % err)
		return false
	return true


func cancel() -> void:
	# Solo se deja de escuchar: el trabajo sigue en el servidor y su resultado
	# queda en caché, así que reintentarlo es gratis.
	_http.cancel_request()
	_busy = false
	_phase = ""
	_job_id = ""


func _fail(motivo: String) -> void:
	_busy = false
	_phase = ""
	failed.emit(motivo)


func _on_completed(result: int, code: int, _headers: PackedStringArray,
		body: PackedByteArray) -> void:
	if result != HTTPRequest.RESULT_SUCCESS:
		_fail("Fallo de red (%d). ¿Está corriendo server.py en %s?" % [result, base_url])
		return

	match _phase:
		"post": _handle_post(code, body)
		"poll": _handle_poll(code, body)
		"audio": _handle_audio(code, body)


func _parse_json(body: PackedByteArray) -> Dictionary:
	var parsed = JSON.parse_string(body.get_string_from_utf8())
	return parsed as Dictionary if typeof(parsed) == TYPE_DICTIONARY else {}


func _handle_post(code: int, body: PackedByteArray) -> void:
	var d := _parse_json(body)
	if code < 200 or code >= 300:
		_fail("El servicio rechazó el payload (%d): %s" % [code, d.get("error", "?")])
		return
	_job_id = String(d.get("job_id", ""))
	if _job_id.is_empty():
		_fail("El servicio no devolvió job_id.")
		return
	started.emit(_job_id)
	_poll_soon(0.0 if bool(d.get("cacheado", false)) else 0.6)


func _poll_soon(delay: float) -> void:
	_phase = "poll"
	if delay > 0.0:
		await get_tree().create_timer(delay).timeout
	if not _busy:
		return
	var err := _http.request(base_url + "/jobs/" + _job_id)
	if err != OK:
		_fail("No se pudo consultar el trabajo (error %d)." % err)


func _handle_poll(code: int, body: PackedByteArray) -> void:
	var d := _parse_json(body)
	if code != 200:
		_fail("El trabajo no se pudo consultar (%d)." % code)
		return

	var estado := String(d.get("estado", "?"))
	_polls += 1
	progress.emit(estado, _polls)

	match estado:
		"listo":
			_info = d
			_phase = "audio"
			var err := _http.request(base_url + "/jobs/" + _job_id + "/audio")
			if err != OK:
				_fail("No se pudo descargar el audio (error %d)." % err)
		"error":
			_fail("El render falló: %s" % d.get("error", "?"))
		_:
			if _polls >= MAX_POLLS:
				_fail("El render tardó demasiado (%d intentos)." % _polls)
			else:
				_poll_soon(POLL_SECONDS)


func _handle_audio(code: int, body: PackedByteArray) -> void:
	if code != 200:
		_fail("La descarga del audio devolvió %d." % code)
		return
	# Tipado como AudioStream, no AudioStreamWAV: la rama de mp3 asigna otro tipo.
	var stream: AudioStream = wav_from_bytes(body)
	if stream == null:
		# Un mp3 solo se puede decodificar en Godot 4.4+ con AudioStreamMP3.
		var mime := String(_info.get("mime", ""))
		if mime.contains("mpeg"):
			var mp3 := AudioStreamMP3.new()
			mp3.data = body
			stream = mp3
		else:
			_fail("No se pudo decodificar el audio recibido (%s)." % mime)
			return
	_busy = false
	_phase = ""
	finished.emit(stream, _info)


## Decodifica un WAV PCM (16 o 8 bits) a AudioStreamWAV.
##
## Godot no trae un decodificador de WAV en tiempo de ejecución: `AudioStreamWAV`
## espera PCM crudo en `data` más el formato aparte, así que hay que recorrer los
## chunks RIFF a mano. Se buscan por nombre en vez de asumir que `fmt ` y `data`
## son los dos primeros: muchos codificadores insertan LIST/INFO en medio.
static func wav_from_bytes(bytes: PackedByteArray) -> AudioStreamWAV:
	if bytes.size() < 44:
		return null
	if bytes.slice(0, 4).get_string_from_ascii() != "RIFF":
		return null
	if bytes.slice(8, 12).get_string_from_ascii() != "WAVE":
		return null

	var channels := 0
	var rate := 0
	var bits := 0
	var pcm := PackedByteArray()
	var i := 12

	while i + 8 <= bytes.size():
		var id := bytes.slice(i, i + 4).get_string_from_ascii()
		var size := bytes.decode_u32(i + 4)
		var start := i + 8
		if size <= 0 or start + size > bytes.size():
			# Tamaño inconsistente: se usa lo que quede en vez de abortar.
			size = bytes.size() - start
		if id == "fmt ":
			if bytes.decode_u16(start) != 1:
				return null                       # solo PCM sin comprimir
			channels = bytes.decode_u16(start + 2)
			rate = int(bytes.decode_u32(start + 4))
			bits = bytes.decode_u16(start + 14)
		elif id == "data":
			pcm = bytes.slice(start, start + size)
		i = start + size + (size & 1)             # los chunks van a bytes pares

	if pcm.is_empty() or channels < 1 or channels > 2 or rate <= 0:
		return null

	var stream := AudioStreamWAV.new()
	match bits:
		16: stream.format = AudioStreamWAV.FORMAT_16_BITS
		8:  stream.format = AudioStreamWAV.FORMAT_8_BITS
		_:  return null
	stream.stereo = channels == 2
	stream.mix_rate = rate
	stream.data = pcm
	return stream
