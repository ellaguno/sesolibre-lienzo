class_name Plant
extends Node3D

## Un objeto colocado: su malla, su luz si la tiene, y su voz.
##
## La forma la construye `MeshLib` a partir del campo `shape` del objeto, así que
## añadir una criatura nueva es una entrada en `Biomes.OBJECTS` más un builder.

const BOB_SPEED := 0.7

var type_id: String
var level: int
var step: int

var _mesh_root: Node3D
var _player: AudioStreamPlayer3D
var _built: MeshLib.Built
var _base_scale := Vector3.ONE
var _phase: float = 0.0
var _bob_amount := 0.035
var _light_base := 0.0
var _flash_tween: Tween


static func create(p_type: String, p_level: int, p_step: int) -> Plant:
	var p := Plant.new()
	p.type_id = p_type
	p.level = p_level
	p.step = p_step
	return p


func _ready() -> void:
	var t: Dictionary = Mapping.TYPES[type_id]
	# El árbol acaba midiendo ~2.5 unidades sobre una isla de 15.5 de ancho. Es
	# más grande de lo "realista" a propósito: la masa del objeto es lo que
	# comunica el registro, y a escala realista no se distingue de un arbusto.
	_base_scale = Vector3.ONE * (0.85 + 0.75 * float(t["size"]))
	_phase = float((level * 31 + step * 17) % 100) / 100.0 * TAU

	_mesh_root = Node3D.new()
	_mesh_root.scale = _base_scale
	add_child(_mesh_root)
	_built = MeshLib.build(String(t["shape"]), String(t["col"]))
	_mesh_root.add_child(_built.root)
	if _built.light != null:
		_light_base = _built.light.light_energy

	_player = AudioStreamPlayer3D.new()
	_player.stream = SoundBank.stream_for(type_id)
	_player.volume_db = float(t["db"])
	# Sin atenuación por distancia: es un instrumento, no un efecto ambiental.
	# Se conserva el paneo para que la escena tenga anchura estéreo — y para que
	# girar la cámara cambie de verdad la perspectiva de escucha.
	_player.attenuation_model = AudioStreamPlayer3D.ATTENUATION_DISABLED
	_player.panning_strength = 1.1
	# Dos voces para que la cola de una nota no se corte al disparar la siguiente.
	_player.max_polyphony = 1 if not bool(t["pitched"]) else 2
	add_child(_player)


## Los objetos que flotan se mecen más que los que están plantados en el suelo.
func set_floating(floating: bool) -> void:
	_bob_amount = 0.12 if floating else 0.035


func _process(delta: float) -> void:
	_phase += delta * BOB_SPEED
	_mesh_root.position.y = sin(_phase) * _bob_amount


func trigger(midi: int, vel_scale := 1.0) -> void:
	_player.pitch_scale = Mapping.pitch_scale(type_id, midi, level)
	_player.volume_db = (float(Mapping.TYPES[type_id]["db"])
			+ Mapping.humanize_db(level, step)
			+ linear_to_db(clampf(vel_scale, 0.05, 1.0)))
	_player.play()

	if _flash_tween != null and _flash_tween.is_valid():
		_flash_tween.kill()
	_flash_tween = create_tween().set_parallel(true)

	# El destello acompaña a la intensidad: si la nota entra suave, el objeto
	# tampoco debe pegar un salto.
	_mesh_root.scale = _base_scale * (1.0 + 0.16 * clampf(vel_scale, 0.2, 1.0))
	_flash_tween.tween_property(_mesh_root, "scale", _base_scale, 0.42) \
		.set_ease(Tween.EASE_OUT).set_trans(Tween.TRANS_BACK)
	# Cada material vuelve a SU emisión base, no a cero: la flor, la medusa y la
	# farola brillan también en reposo.
	for i in _built.materials.size():
		_built.materials[i].emission_energy_multiplier = _built.emissions[i] + 1.5
		_flash_tween.tween_property(_built.materials[i],
			"emission_energy_multiplier", _built.emissions[i], 0.5)
	if _built.light != null:
		# Se usa el valor guardado, no el actual: releerlo durante un destello
		# anterior haría crecer el brillo base a cada disparo.
		_built.light.light_energy = _light_base + 1.7
		_flash_tween.tween_property(_built.light, "light_energy", _light_base, 0.6)
