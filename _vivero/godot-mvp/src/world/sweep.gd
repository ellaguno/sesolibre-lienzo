class_name Sweep
extends Node3D

## El barrido: recorre el ciclo y hace sonar lo que toca. Es la pieza que hace
## legible el mapeo — el jugador ve qué suena y por qué en vez de deducirlo.
##
## Dos formas según el layout: **cuña** giratoria para el disco, **barra**
## vertical que atraviesa la escena para la pared y la retícula.

const SEGMENTS := 20
const TRAIL := 0.62          ## radianes de cola (solo cuña)

var _layout: Layouts.Base
var _mesh: MeshInstance3D
var _edge: MeshInstance3D
var _mat: StandardMaterial3D
var _edge_mat: StandardMaterial3D
var _fill_alpha := 1.0


func setup(layout: Layouts.Base) -> void:
	_layout = layout
	for c in get_children():
		c.queue_free()

	_mat = _additive()
	_edge_mat = _additive()
	_edge_mat.vertex_color_use_as_albedo = false

	_mesh = MeshInstance3D.new()
	_mesh.material_override = _mat
	add_child(_mesh)
	_edge = MeshInstance3D.new()
	_edge.material_override = _edge_mat
	add_child(_edge)

	var span := layout.sweep_span()
	# La cuña lleva el degradado en sus colores de vértice, así que su albedo va
	# a alfa 1: si no, se multiplican los dos y el barrido desaparece. La barra
	# es una caja con vértices blancos, y ahí el alfa del albedo es lo único que
	# controla la transparencia.
	_fill_alpha = 1.0 if layout.sweep_kind() in ["wedge", "arc"] else 0.16
	if layout.sweep_kind() == "arc":
		# Meridiano: una cinta que sube por la cúpula del horizonte al cenit.
		_mesh.mesh = _build_arc(span.x, span.y, span.z)
		_edge.mesh = _build_arc(span.x * 1.002, span.y, span.z, 0.012, true)
	elif layout.sweep_kind() == "wedge":
		_mesh.mesh = _build_wedge(span.x)
		var bar := BoxMesh.new()
		bar.size = Vector3(span.x, span.y, span.z)
		_edge.mesh = bar
		_edge.position = Vector3(span.x * 0.5, 0.01, 0.0)
	else:
		# Barra: una lámina fina que atraviesa la escena, con el canto encendido.
		var slab := BoxMesh.new()
		slab.size = span
		_mesh.mesh = slab
		_mesh.position = Vector3.ZERO
		var line := BoxMesh.new()
		line.size = Vector3(span.x * 0.22, span.y, span.z * 0.22)
		_edge.mesh = line
		_edge.position = Vector3.ZERO


func _process(_delta: float) -> void:
	if _layout == null:
		return
	transform = _layout.sweep_xform(Conductor.cycle_progress())


func set_mood(mood_key: String) -> void:
	var accent := Color(String(Biomes.MOODS[mood_key]["sun"]))
	if _mat == null:
		return
	_mat.albedo_color = Color(accent.r, accent.g, accent.b, _fill_alpha)
	_edge_mat.albedo_color = accent
	_edge_mat.emission = accent


func _additive() -> StandardMaterial3D:
	var m := StandardMaterial3D.new()
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	m.blend_mode = BaseMaterial3D.BLEND_MODE_ADD
	m.cull_mode = BaseMaterial3D.CULL_DISABLED
	# Sin escritura de profundidad: si no, el barrido tapa lo que cruza.
	m.depth_draw_mode = BaseMaterial3D.DEPTH_DRAW_DISABLED
	m.vertex_color_use_as_albedo = true
	m.emission_enabled = true
	m.emission_energy_multiplier = 1.0
	return m


## Cinta a lo largo de un meridiano de la cúpula.
func _build_arc(radius: float, el0: float, el1: float,
		width := 0.035, solid := false) -> ArrayMesh:
	var verts := PackedVector3Array()
	var colors := PackedColorArray()
	var n := 24
	for i in n:
		var t0 := float(i) / float(n)
		var t1 := float(i + 1) / float(n)
		var e0 := lerpf(el0, el1, t0)
		var e1 := lerpf(el0, el1, t1)
		# Cuatro esquinas del segmento, desplazadas en azimut para dar ancho.
		var p00 := _on_sphere(radius, e0, -width)
		var p01 := _on_sphere(radius, e0, width)
		var p10 := _on_sphere(radius, e1, -width)
		var p11 := _on_sphere(radius, e1, width)
		# 0.12: sobre un cielo negro y en mezcla aditiva, cualquier valor más
		# alto convierte el meridiano en una pared luminosa.
		var a0 := Color(1, 1, 1, 1.0 if solid else 0.12)
		var a1 := Color(1, 1, 1, 1.0 if solid else 0.12)
		for v in [p00, p01, p11, p00, p11, p10]:
			verts.append(v)
		for c in [a0, a0, a1, a0, a1, a1]:
			colors.append(c)
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_COLOR] = colors
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


static func _on_sphere(r: float, el: float, az: float) -> Vector3:
	return Vector3(r * cos(el) * cos(az), r * sin(el), r * cos(el) * sin(az))


## Abanico de triángulos desde el centro, con la opacidad cayendo hacia la cola.
func _build_wedge(radius: float) -> ArrayMesh:
	var verts := PackedVector3Array()
	var colors := PackedColorArray()
	for i in SEGMENTS:
		var t0 := float(i) / float(SEGMENTS)
		var t1 := float(i + 1) / float(SEGMENTS)
		var a0 := -t0 * TRAIL
		var a1 := -t1 * TRAIL
		verts.append(Vector3.ZERO)
		verts.append(Vector3(cos(a0), 0.0, sin(a0)) * radius)
		verts.append(Vector3(cos(a1), 0.0, sin(a1)) * radius)
		colors.append(Color(1, 1, 1, 0.16))
		colors.append(Color(1, 1, 1, 0.20 * (1.0 - t0)))
		colors.append(Color(1, 1, 1, 0.20 * (1.0 - t1)))

	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_COLOR] = colors
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh
