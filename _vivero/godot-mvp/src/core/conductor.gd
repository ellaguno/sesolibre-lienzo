extends Node

## Reloj musical. Autoload `Conductor`.
##
## El tiempo se deriva de `Time.get_ticks_usec()` acumulando *pulsos*, no
## segundos, para que cambiar el BPM no dé un salto ni acumule error de float.
##
## LIMITACIÓN CONOCIDA: los disparos se resuelven en `_process`, así que su
## precisión está atada al frame (~16 ms a 60 fps). Godot no ofrece "reproduce
## este sample en el instante T" desde el hilo de audio. Para producción esto se
## sustituye por FMOD/Wwise o por un AudioStreamPlayback propio; en móvil de
## gama baja, además, la latencia de salida de Android puede sumar 40-80 ms.
## Es el riesgo técnico número uno del proyecto y este MVP no lo resuelve:
## solo permite comprobar si el mapeo funciona.

signal step_advanced(slot: int, cycle: int)
signal cycle_started()

const MAX_CATCHUP_STEPS := 4

var playing := false

## Pasos por ciclo. Lo fija el bioma: no todos los ambientes tienen por qué
## repartir el compás en dieciséis.
var steps_per_cycle := 16:
	set(value):
		steps_per_cycle = maxi(2, value)

## Duración de un paso, en pulsos. La fija el bioma: una ciudad cuenta en
## semicorcheas y un cielo en negras, y eso cambia el carácter más que el tempo.
var step_beats := Mapping.STEP_BEATS:
	set(value):
		_beats_at_origin = beats()
		_origin_usec = Time.get_ticks_usec()
		step_beats = maxf(0.0625, value)

var bpm: float = 68.0:
	set(value):
		# Se ancla el reloj antes de cambiar el tempo para no saltar de posición.
		_beats_at_origin = beats()
		_origin_usec = Time.get_ticks_usec()
		bpm = maxf(20.0, value)

var _origin_usec: int = 0
var _beats_at_origin: float = 0.0
var _last_step: int = -1


func _ready() -> void:
	_origin_usec = Time.get_ticks_usec()


func beats() -> float:
	if not playing:
		return _beats_at_origin
	var elapsed := float(Time.get_ticks_usec() - _origin_usec) / 1_000_000.0
	return _beats_at_origin + elapsed * bpm / 60.0


func cycle_beats() -> float:
	return step_beats * float(steps_per_cycle)


## 0..1 dentro del ciclo. Lo usa el barrido para dibujarse.
func cycle_progress() -> float:
	return fposmod(beats(), cycle_beats()) / cycle_beats()


func play() -> void:
	if playing:
		return
	_origin_usec = Time.get_ticks_usec()
	playing = true


func pause() -> void:
	if not playing:
		return
	_beats_at_origin = beats()
	playing = false


func toggle() -> void:
	if playing:
		pause()
	else:
		play()


func cycle_index() -> int:
	return int(floor(beats() / cycle_beats()))


func reset() -> void:
	_beats_at_origin = 0.0
	_origin_usec = Time.get_ticks_usec()
	_last_step = -1


func _process(_delta: float) -> void:
	if not playing:
		return
	var target := int(floor(beats() / Mapping.STEP_BEATS))
	# Tras un stall (carga de escena, cambio de app) no tiene sentido disparar
	# cincuenta pasos de golpe: se salta al presente.
	if target - _last_step > MAX_CATCHUP_STEPS:
		_last_step = target - 1
	while _last_step < target:
		_last_step += 1
		var slot := _last_step % steps_per_cycle
		if slot == 0:
			cycle_started.emit()
		step_advanced.emit(slot)
