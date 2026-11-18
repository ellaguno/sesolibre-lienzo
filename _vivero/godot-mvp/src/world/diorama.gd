class_name Diorama
extends Node3D

## La escena 3D: base, rejilla, barrido, objetos y luz.
##
## Es el espejo del `Garden`. El Garden es la verdad; el Diorama la representa y
## traduce toques en coordenadas de celda. Toda la geometría se delega en el
## `Layouts.Base` del bioma, así que cambiar de disco a acuario no toca nada de
## aquí salvo reconstruir.

signal cell_tapped(level: int, step: int, occupied: bool)

const DOT_RADIUS := 0.075

var garden: Garden
var layout: Layouts.Base

var _plants: Dictionary = {}          ## Vector2i -> Plant
var _sweep: Sweep
var _dots: MultiMeshInstance3D
var _base_nodes: Array[Node3D] = []
var _ground_mats: Array[StandardMaterial3D] = []
var _grid_mats: Array[StandardMaterial3D] = []
var _sun: DirectionalLight3D
var _env: WorldEnvironment
var _appreciating := false
## Duración del ciclo en segundos. La necesita el sistema de variación: el
## intervalo entre repeticiones se mide en tiempo, no en vueltas.
var _cycle_seconds := 7.0


func setup(p_garden: Garden) -> void:
	garden = p_garden
	_build_environment()
	_sweep = Sweep.new()
	add_child(_sweep)
	rebuild_biome()

	Conductor.step_advanced.connect(_on_step)
	garden.changed.connect(_sync_plants)
	garden.biome_changed.connect(rebuild_biome)


## Reconstruye todo lo que depende del bioma: layout, base, rejilla y barrido.
func rebuild_biome() -> void:
	var b := Biomes.biome(garden.biome_id)
	layout = Layouts.make(String(b["layout"]), garden.levels(), garden.steps())

	for n in _base_nodes:
		n.queue_free()
	_base_nodes.clear()
	_ground_mats.clear()
	_grid_mats.clear()
	if _dots != null:
		_dots.queue_free()

	var glow := Biomes.ground_glow(garden.biome_id)
	for node in layout.build_base(self, Biomes.ground_color(
			garden.biome_id, garden.mood_key).to_html(false)):
		_base_nodes.append(node)
		if node.has_meta("ground_mat"):
			var gm: StandardMaterial3D = node.get_meta("ground_mat")
			gm.emission_enabled = glow > 0.0
			gm.emission_energy_multiplier = glow
			_ground_mats.append(gm)
		if node.has_meta("grid_mat"):
			_grid_mats.append(node.get_meta("grid_mat"))

	_refresh_cycle_seconds()
	_build_dots()
	_sweep.setup(layout)
	_sync_plants()
	set_mood(garden.mood_key)
	set_appreciation(_appreciating)


func _build_environment() -> void:
	var env := Environment.new()
	env.background_mode = Environment.BG_COLOR
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	# Con el ambiente a 1.0 la luz difusa se come al sol y el momento del día
	# deja de leerse. El sol tiene que ser quien define la escena.
	env.ambient_light_energy = 0.45
	env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	# ADITIVO, no el SOFTLIGHT por defecto: sobre un fondo oscuro el softlight
	# levanta toda la imagen a pastel. Y glow_bloom a 0 — cualquier valor por
	# encima hace brillar TODOS los píxeles, no solo los que pasan el umbral.
	env.glow_enabled = true
	env.glow_blend_mode = Environment.GLOW_BLEND_MODE_ADDITIVE
	env.glow_intensity = 0.45
	env.glow_bloom = 0.0
	env.glow_hdr_threshold = 1.0
	_env = WorldEnvironment.new()
	_env.environment = env
	add_child(_env)

	_sun = DirectionalLight3D.new()
	_sun.rotation = Vector3(deg_to_rad(-52.0), deg_to_rad(38.0), 0.0)
	_sun.shadow_enabled = true
	_sun.directional_shadow_mode = DirectionalLight3D.SHADOW_ORTHOGONAL
	_sun.directional_shadow_max_distance = 48.0
	add_child(_sun)


## Puntos de celda libre. Un MultiMesh en vez de 80 nodos: es la diferencia
## entre 80 draw calls y una.
func _build_dots() -> void:
	var dot := SphereMesh.new()
	dot.radius = DOT_RADIUS
	dot.height = DOT_RADIUS * 2.0
	dot.radial_segments = 8
	dot.rings = 4

	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.albedo_color = Color(1, 1, 1, 0.22)

	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = dot
	mm.instance_count = layout.cell_count()

	_dots = MultiMeshInstance3D.new()
	_dots.multimesh = mm
	_dots.material_override = mat
	add_child(_dots)


func _sync_plants() -> void:
	for k in _plants.keys():
		if not garden.cells.has(k):
			(_plants[k] as Plant).queue_free()
			_plants.erase(k)
	var floating := String(Biomes.biome(garden.biome_id)["layout"]) == "wall"
	for k in garden.cells:
		var cell := k as Vector2i
		var type_id := String(garden.cells[k])
		var existing: Plant = _plants.get(cell)
		if existing != null and existing.type_id == type_id:
			continue
		if existing != null:
			existing.queue_free()
		var p := Plant.create(type_id, cell.x, cell.y)
		p.position = layout.cell_position(cell)
		p.rotation.y = layout.cell_rotation(cell)
		_plants[cell] = p
		add_child(p)
		p.set_floating(floating)
	_refresh_dots()


func _refresh_dots() -> void:
	var mm := _dots.multimesh
	var i := 0
	for lv in garden.levels():
		for st in garden.steps():
			var cell := Vector2i(lv, st)
			var xf := Transform3D(Basis.IDENTITY, layout.cell_position(cell))
			# Celda ocupada: se escala a cero en vez de reconstruir el MultiMesh.
			if garden.cells.has(cell):
				xf = xf.scaled_local(Vector3.ZERO)
			mm.set_instance_transform(i, xf)
			i += 1


func _on_step(step: int, cycle: int) -> void:
	for cell in garden.at_step(step):
		var p: Plant = _plants.get(cell)
		if p == null:
			continue
		# No todo suena en cada vuelta: es lo que impide que la pieza se repita
		# literalmente cada ciclo.
		if not Mapping.should_trigger(garden.biome_id, p.type_id, p.level, p.step,
				cycle, _cycle_seconds):
			continue
		p.trigger(Mapping.midi_of(garden.biome_id, p.type_id, p.level,
				garden.mood_key, garden.transpose),
				Mapping.velocity_scale(p.level, p.step, cycle, _cycle_seconds))


func _refresh_cycle_seconds() -> void:
	_cycle_seconds = (Mapping.cycle_beats(garden.biome_id, garden.steps())
			* 60.0 / maxf(garden.bpm, 1.0))


func set_mood(mood_key: String) -> void:
	_refresh_cycle_seconds()
	var m: Dictionary = Biomes.MOODS[mood_key]
	var sky := Color(String(m["sky"]))
	var amb := Color(String(m["amb"]))
	var sun := Color(String(m["sun"]))
	var ground := Biomes.ground_color(garden.biome_id, mood_key)

	var env := _env.environment
	var tw := create_tween().set_parallel(true)
	tw.tween_property(env, "background_color", sky, 1.4)
	tw.tween_property(env, "ambient_light_color", amb, 1.4)
	tw.tween_property(_sun, "light_color", sun, 1.4)
	tw.tween_property(_sun, "light_energy", float(m["sun_energy"]), 1.4)
	for gm in _ground_mats:
		tw.tween_property(gm, "albedo_color", ground, 1.4)
		if gm.emission_enabled:
			tw.tween_property(gm, "emission", ground, 1.4)
	_sweep.set_mood(mood_key)


func set_appreciation(on: bool) -> void:
	# En apreciación la rejilla desaparece: no se está construyendo, se está
	# mirando. El barrido se queda, porque es lo que hace visible la música.
	_appreciating = on
	if _dots != null:
		_dots.visible = not on
	for m in _grid_mats:
		m.albedo_color = Color(1, 1, 1, 0.0 if on else 0.055)


# --------------------------------------------------------------------------
# entrada
# --------------------------------------------------------------------------

## El layout decide contra qué superficie se proyecta el toque.
func pick(from: Vector3, dir: Vector3) -> Dictionary:
	return layout.pick(from, dir)


func handle_tap(point: Vector3) -> void:
	var c := layout.cell_at(point)
	if not c["has"]:
		return
	var lv := int(c["ring"])
	var st := int(c["slot"])
	cell_tapped.emit(lv, st, garden.cells.has(Vector2i(lv, st)))


## Dispara un objeto al colocarlo, sin esperar al barrido. El feedback inmediato
## es la mitad de la sensación de plantar.
func preview(level: int, step: int) -> void:
	var p: Plant = _plants.get(Vector2i(level, step))
	if p == null:
		return
	# La vista previa suena siempre a plena intensidad: es feedback del gesto,
	# no parte de la pieza.
	p.trigger(Mapping.midi_of(garden.biome_id, p.type_id, level,
			garden.mood_key, garden.transpose), 1.0)
