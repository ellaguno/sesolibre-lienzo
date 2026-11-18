class_name OrbitCamera
extends Camera3D

## Cámara orbital restringida sobre el diorama.
##
## Nunca hay cámara libre: solo órbita, inclinación acotada y zoom. Eso es lo
## que hace que colocar objetos con un toque no tenga ambigüedad de perspectiva
## — el problema que hunde a la mayoría de los constructores 3D en móvil.

## Los límites los fija el layout: una pared vertical necesita mirarse casi de
## frente y un disco casi desde arriba. Estos son solo los valores de partida.
var pitch_min := -1.33
var pitch_max := -0.20
var dist_min := 11.0
var dist_max := 30.0
const AUTO_SPEED := 0.045
const FOV := 55.0

## Punto de mira por debajo del suelo a propósito. En perspectiva, el borde
## cercano del disco proyecta mucho más bajo que el lejano, así que la silueta
## visible queda descentrada hacia abajo; bajar la mira la sube en el encuadre
## sin tocar el ángulo de la cámara.
var target := Vector3(0.0, -1.6, 0.0)
var yaw: float = 0.7
var pitch: float = -0.72
var dist: float = 16.0
var auto_orbit := false
var _home_pitch := -0.72
var _home_dist := 16.0

var _yaw_goal: float = 0.7
var _pitch_goal: float = -0.72
var _dist_goal: float = 16.0


func _ready() -> void:
	_yaw_goal = yaw
	_pitch_goal = pitch
	_dist_goal = dist
	fov = FOV
	# KEEP_WIDTH es obligatorio en retrato. Con el KEEP_HEIGHT por defecto, `fov`
	# se aplica al eje vertical y en una pantalla 9:18 el campo horizontal se
	# estrecha tanto que el disco (que es ancho) se sale del encuadre.
	keep_aspect = Camera3D.KEEP_WIDTH
	_apply()


func _process(delta: float) -> void:
	if auto_orbit:
		_yaw_goal += delta * AUTO_SPEED
	# Suavizado exponencial independiente del framerate. `yaw` se interpola sin
	# envolver (no lerp_angle): el objetivo es acumulativo, y envolverlo haría
	# que la órbita automática diera un tirón cada vuelta.
	var k := 1.0 - exp(-delta * 11.0)
	yaw = lerp(yaw, _yaw_goal, k)
	pitch = lerp(pitch, _pitch_goal, k)
	dist = lerp(dist, _dist_goal, k)
	_apply()


func orbit(delta_px: Vector2) -> void:
	_yaw_goal -= delta_px.x * 0.006
	_pitch_goal = clampf(_pitch_goal - delta_px.y * 0.005, pitch_min, pitch_max)


func zoom(factor: float) -> void:
	_dist_goal = clampf(_dist_goal * factor, dist_min, dist_max)


## Aplica el encuadre que pide el layout del bioma.
func apply_layout(layout: Layouts.Base) -> void:
	var d := layout.camera_defaults()
	target = layout.camera_target()
	pitch_min = float(d["pitch_min"])
	pitch_max = float(d["pitch_max"])
	_yaw_goal = float(d["yaw"])
	_pitch_goal = clampf(float(d["pitch"]), pitch_min, pitch_max)
	_dist_goal = float(d["dist"])
	dist_min = float(d["dist_min"]) if d.has("dist_min") else _dist_goal * 0.6
	dist_max = float(d["dist_max"]) if d.has("dist_max") else _dist_goal * 1.9
	_home_pitch = _pitch_goal
	_home_dist = _dist_goal
	yaw = _yaw_goal
	pitch = _pitch_goal
	dist = _dist_goal
	_apply()


## 0 = mirando a ras de la escena, 1 = cenital. Lo consume SoundBank para que la
## cámara sea también la posición de escucha.
func top_down_amount() -> float:
	return clampf(inverse_lerp(pitch_max, pitch_min, pitch), 0.0, 1.0)


func distance_amount() -> float:
	return clampf(inverse_lerp(dist_min, dist_max, dist), 0.0, 1.0)


func frame_for_appreciation(on: bool) -> void:
	auto_orbit = on
	# En apreciación el diorama tiene que llenar el encuadre: se acerca hasta que
	# los bordes de la isla sangran fuera de pantalla.
	_pitch_goal = clampf(_home_pitch + 0.12, pitch_min, pitch_max) if on else _home_pitch
	_dist_goal = _home_dist * (0.84 if on else 1.0)


func _apply() -> void:
	var offset := Vector3(
		cos(pitch) * cos(yaw),
		-sin(pitch),
		cos(pitch) * sin(yaw)
	) * dist
	position = target + offset
	look_at(target, Vector3.UP)
