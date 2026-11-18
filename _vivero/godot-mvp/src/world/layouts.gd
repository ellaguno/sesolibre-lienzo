class_name Layouts
extends RefCounted

## Cómo se reparten las celdas en el espacio.
##
## Una celda es siempre `Vector2i(nivel, paso)`:
##   `paso`  -> posición en el ciclo (lo que el barrido recorre)
##   `nivel` -> grado de la escala (grave abajo/dentro, agudo arriba/fuera)
##
## Lo que cambia entre layouts es la GEOMETRÍA, no el mapeo musical. Por eso un
## bioma puede cambiar de disco a pared sin tocar una línea del secuenciador.
##
## El disco radial es solo uno de los tres: sirve al jardín y a la selva, pero un
## acuario quiere una pared vertical (profundidad = altura) y una ciudad quiere
## una retícula en el suelo por la que pasa una línea.


class Base extends RefCounted:
	var levels := 5
	var steps := 16

	func _init(p_levels: int, p_steps: int) -> void:
		levels = p_levels
		steps = p_steps

	func cell_count() -> int:
		return levels * steps

	## Posición del centro de la celda.
	func cell_position(_cell: Vector2i) -> Vector3:
		return Vector3.ZERO

	## Giro en Y del objeto, para que la escena no parezca una rejilla.
	func cell_rotation(_cell: Vector2i) -> float:
		return 0.0

	## Plano sobre el que se proyecta el toque (los layouts planos lo usan).
	func pick_plane() -> Plane:
		return Plane(Vector3.UP, 0.0)

	## Punto de la superficie de trabajo bajo un rayo de pantalla.
	##
	## Cada layout decide contra QUÉ se proyecta: un plano horizontal para el
	## disco y la retícula, uno vertical para el acuario, una esfera para la
	## cúpula. Antes esto era un `Plane` fijo y una cúpula no cabía.
	func pick(from: Vector3, dir: Vector3) -> Dictionary:
		var hit = pick_plane().intersects_ray(from, dir)
		if hit == null:
			return {"has": false, "point": Vector3.ZERO}
		return {"has": true, "point": hit as Vector3}

	## Celda bajo un punto del plano: {has, ring, slot}.
	func cell_at(_point: Vector3) -> Dictionary:
		return {"has": false}

	## "wedge" (cuña giratoria) o "bar" (barra que recorre un eje).
	func sweep_kind() -> String:
		return "wedge"

	## Dónde va el barrido para un progreso 0..1 del ciclo.
	func sweep_xform(_t: float) -> Transform3D:
		return Transform3D.IDENTITY

	## Volumen del barrido en los tres ejes. Para la cuña solo cuenta X (radio).
	## Declararlo en 3D evita el error de darle al barrido tanta profundidad como
	## altura: se convierte en una losa opaca que tapa la escena.
	func sweep_span() -> Vector3:
		return Vector3(0.3, 6.0, 6.0)

	## Punto al que mira la cámara.
	func camera_target() -> Vector3:
		return Vector3(0.0, -1.6, 0.0)

	## Radio aproximado de la escena, para encuadrar la cámara.
	func extent() -> float:
		return 8.0

	## yaw / pitch / dist iniciales y límites de inclinación.
	func camera_defaults() -> Dictionary:
		return {"yaw": 0.7, "pitch": -0.72, "dist": 16.0,
				"pitch_min": -1.33, "pitch_max": -0.20}

	## Malla del suelo/fondo. Devuelve los nodos creados.
	func build_base(_parent: Node3D, _hex: String) -> Array[Node3D]:
		return []

	## Posiciones de los puntos de celda libre.
	func dot_transforms() -> Array[Transform3D]:
		var out: Array[Transform3D] = []
		for lv in levels:
			for st in steps:
				out.append(Transform3D(Basis.IDENTITY, cell_position(Vector2i(lv, st))))
		return out


# ==========================================================================
# RADIAL — disco. Jardín, selva.
#   paso  = ángulo    nivel = anillo
# ==========================================================================

class Radial extends Base:
	const R0 := 2.6
	const DR := 1.05

	func ring_radius(ring: int) -> float:
		return R0 + ring * DR

	func extent() -> float:
		return ring_radius(levels - 1)

	func cell_position(cell: Vector2i) -> Vector3:
		var a := float(cell.y) / float(steps) * TAU
		var r := ring_radius(cell.x)
		return Vector3(r * cos(a), 0.0, r * sin(a))

	func cell_rotation(cell: Vector2i) -> float:
		return -float(cell.y) / float(steps) * TAU

	func cell_at(point: Vector3) -> Dictionary:
		var r := Vector2(point.x, point.z).length()
		if r < R0 - DR * 0.6 or r > extent() + DR * 0.6:
			return {"has": false}
		var ring := clampi(int(round((r - R0) / DR)), 0, levels - 1)
		var slot := posmod(int(round(atan2(point.z, point.x) / TAU * float(steps))), steps)
		return {"has": true, "ring": ring, "slot": slot}

	func sweep_kind() -> String:
		return "wedge"

	func sweep_xform(t: float) -> Transform3D:
		return Transform3D(Basis(Vector3.UP, -t * TAU), Vector3(0, 0.03, 0))

	func sweep_span() -> Vector3:
		return Vector3(extent() + DR * 0.45, 0.012, 0.05)

	func build_base(parent: Node3D, hex: String) -> Array[Node3D]:
		var out: Array[Node3D] = []
		var r := extent()
		var mat := StandardMaterial3D.new()
		mat.albedo_color = Color(hex)
		mat.roughness = 0.95
		# Ligeramente cónica: el borde inferior más estrecho da la lectura de
		# "diorama flotante" en vez de "mesa".
		var body := CylinderMesh.new()
		body.top_radius = r + 0.95
		body.bottom_radius = r + 0.45
		body.height = 0.85
		body.radial_segments = 64
		var mi := MeshInstance3D.new()
		mi.mesh = body
		mi.material_override = mat
		mi.position.y = -0.425
		mi.set_meta("ground_mat", mat)
		parent.add_child(mi)
		out.append(mi)

		for i in levels:
			var ring := TorusMesh.new()
			ring.inner_radius = ring_radius(i) - 0.012
			ring.outer_radius = ring_radius(i) + 0.012
			ring.rings = 64
			ring.ring_segments = 4
			out.append(Layouts._grid_mesh(parent, ring, Vector3(0, 0.004, 0)))
		return out


# ==========================================================================
# WALL — pared vertical. Acuario, mar, cielo.
#   paso = X (izquierda a derecha)    nivel = Y (profundidad = altura)
# ==========================================================================

class Wall extends Base:
	const DX := 0.95
	const DY := 1.05
	const Y0 := 1.1

	func half_width() -> float:
		return (float(steps) - 1.0) * DX * 0.5

	func extent() -> float:
		return maxf(half_width(), Y0 + float(levels - 1) * DY)

	func cell_position(cell: Vector2i) -> Vector3:
		return Vector3(-half_width() + float(cell.y) * DX, Y0 + float(cell.x) * DY, 0.0)

	func pick_plane() -> Plane:
		# El plano de trabajo es vertical (z = 0), mirado de frente.
		return Plane(Vector3.BACK, 0.0)

	func cell_at(point: Vector3) -> Dictionary:
		var st := int(round((point.x + half_width()) / DX))
		var lv := int(round((point.y - Y0) / DY))
		if st < 0 or st >= steps or lv < 0 or lv >= levels:
			return {"has": false}
		# Margen de media celda: fuera de la retícula no se planta.
		if absf(point.x - cell_position(Vector2i(lv, st)).x) > DX * 0.6:
			return {"has": false}
		if absf(point.y - cell_position(Vector2i(lv, st)).y) > DY * 0.6:
			return {"has": false}
		return {"has": true, "ring": lv, "slot": st}

	func sweep_kind() -> String:
		return "bar"

	func sweep_xform(t: float) -> Transform3D:
		var x := -half_width() - DX * 0.5 + t * (float(steps) * DX)
		return Transform3D(Basis.IDENTITY, Vector3(x, Y0 + float(levels - 1) * DY * 0.5, 0))

	func sweep_span() -> Vector3:
		# Fino en Z: la barra atraviesa el tanque, no lo tapa.
		return Vector3(DX * 0.30, float(levels) * DY + 0.9, 0.55)

	func camera_target() -> Vector3:
		# El centro del tanque, no el suelo: si no, la pared queda arriba.
		return Vector3(0.0, Y0 + float(levels - 1) * DY * 0.5, 0.0)

	func camera_defaults() -> Dictionary:
		# Casi de frente: una pared vista desde arriba no se lee.
		return {"yaw": PI * 0.5, "pitch": -0.14, "dist": 16.5,
				"pitch_min": -0.75, "pitch_max": 0.30}

	func build_base(parent: Node3D, hex: String) -> Array[Node3D]:
		var out: Array[Node3D] = []
		var w := half_width() * 2.0 + DX * 1.6
		var h := float(levels) * DY + Y0
		var mat := StandardMaterial3D.new()
		mat.albedo_color = Color(hex)
		mat.roughness = 0.9
		var back := BoxMesh.new()
		back.size = Vector3(w, h, 0.5)
		var mi := MeshInstance3D.new()
		mi.mesh = back
		mi.material_override = mat
		mi.position = Vector3(0, h * 0.5 - 0.1, -1.2)
		mi.set_meta("ground_mat", mat)
		parent.add_child(mi)
		out.append(mi)

		# Suelo del tanque: ancla la escena, si no los objetos flotan sin contexto.
		var floor_mesh := BoxMesh.new()
		floor_mesh.size = Vector3(w, 0.35, 2.6)
		var fl := MeshInstance3D.new()
		fl.mesh = floor_mesh
		fl.material_override = mat
		fl.position = Vector3(0, -0.1, -0.2)
		parent.add_child(fl)
		out.append(fl)

		for i in levels:
			var line := BoxMesh.new()
			line.size = Vector3(w - DX, 0.012, 0.012)
			out.append(Layouts._grid_mesh(parent, line,
				Vector3(0, Y0 + float(i) * DY, 0)))
		return out


# ==========================================================================
# FIELD — retícula en el suelo. Ciudad, interior.
#   paso = X    nivel = Z (profundidad)   los objetos crecen en Y
# ==========================================================================

class Field extends Base:
	const DX := 1.15
	const DZ := 1.25

	func half_width() -> float:
		return (float(steps) - 1.0) * DX * 0.5

	func half_depth() -> float:
		return (float(levels) - 1.0) * DZ * 0.5

	func extent() -> float:
		return maxf(half_width(), half_depth())

	func cell_position(cell: Vector2i) -> Vector3:
		return Vector3(-half_width() + float(cell.y) * DX, 0.0,
					   -half_depth() + float(cell.x) * DZ)

	func cell_at(point: Vector3) -> Dictionary:
		var st := int(round((point.x + half_width()) / DX))
		var lv := int(round((point.z + half_depth()) / DZ))
		if st < 0 or st >= steps or lv < 0 or lv >= levels:
			return {"has": false}
		var c := cell_position(Vector2i(lv, st))
		if absf(point.x - c.x) > DX * 0.6 or absf(point.z - c.z) > DZ * 0.6:
			return {"has": false}
		return {"has": true, "ring": lv, "slot": st}

	func sweep_kind() -> String:
		return "bar"

	func sweep_xform(t: float) -> Transform3D:
		var x := -half_width() - DX * 0.5 + t * (float(steps) * DX)
		return Transform3D(Basis.IDENTITY, Vector3(x, 0.85, 0))

	func sweep_span() -> Vector3:
		# Fino en X (eje de avance) y profundo en Z: cruza la retícula a lo ancho.
		return Vector3(DX * 0.26, 1.7, float(levels) * DZ + 1.3)

	func camera_target() -> Vector3:
		return Vector3(0.0, -0.5, 0.0)

	func camera_defaults() -> Dictionary:
		return {"yaw": 0.9, "pitch": -0.62, "dist": 20.0,
				"pitch_min": -1.25, "pitch_max": -0.16}

	func build_base(parent: Node3D, hex: String) -> Array[Node3D]:
		var out: Array[Node3D] = []
		var mat := StandardMaterial3D.new()
		mat.albedo_color = Color(hex)
		mat.roughness = 0.95
		var slab := BoxMesh.new()
		slab.size = Vector3(half_width() * 2.0 + DX * 1.8, 0.7,
							half_depth() * 2.0 + DZ * 1.8)
		var mi := MeshInstance3D.new()
		mi.mesh = slab
		mi.material_override = mat
		mi.position.y = -0.35
		mi.set_meta("ground_mat", mat)
		parent.add_child(mi)
		out.append(mi)

		for i in levels:
			var line := BoxMesh.new()
			line.size = Vector3(half_width() * 2.0 + DX, 0.012, 0.012)
			out.append(Layouts._grid_mesh(parent, line,
				Vector3(0, 0.004, -half_depth() + float(i) * DZ)))
		return out


# --------------------------------------------------------------------------

## Malla de rejilla: siempre translúcida y sin sombreado, para que se pueda
## desvanecer en modo apreciación sin tocar el resto.
static func _grid_mesh(parent: Node3D, mesh: Mesh, pos: Vector3) -> MeshInstance3D:
	var m := StandardMaterial3D.new()
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	m.albedo_color = Color(1, 1, 1, 0.055)
	var mi := MeshInstance3D.new()
	mi.mesh = mesh
	mi.material_override = m
	mi.position = pos
	mi.set_meta("grid_mat", m)
	parent.add_child(mi)
	return mi


# ==========================================================================
# DOME — cúpula. Cielo, estrellas.
#   paso = azimut    nivel = elevación (horizonte -> cenit)
# ==========================================================================

class Dome extends Base:
	const R := 8.2
	const EL0 := 0.20      ## elevación del primer nivel, en radianes
	const EL1 := 1.24      ## casi el cenit

	func _el(level: int) -> float:
		if levels <= 1:
			return EL0
		return EL0 + (EL1 - EL0) * float(level) / float(levels - 1)

	func _az(step: int) -> float:
		return float(step) / float(steps) * TAU

	func extent() -> float:
		return R

	func cell_position(cell: Vector2i) -> Vector3:
		var el := _el(cell.x)
		var az := _az(cell.y)
		return Vector3(R * cos(el) * cos(az), R * sin(el), R * cos(el) * sin(az))

	func cell_rotation(cell: Vector2i) -> float:
		return -_az(cell.y)

	## Intersección rayo-esfera. La cámara está fuera, así que interesa la raíz
	## menor; si esa cae en el hemisferio inferior se prueba la otra.
	func pick(from: Vector3, dir: Vector3) -> Dictionary:
		var b := 2.0 * from.dot(dir)
		var c := from.length_squared() - R * R
		var disc := b * b - 4.0 * c
		if disc < 0.0:
			return {"has": false, "point": Vector3.ZERO}
		var sq := sqrt(disc)
		for t in [(-b - sq) * 0.5, (-b + sq) * 0.5]:
			if t <= 0.0:
				continue
			var p: Vector3 = from + dir * t
			if p.y >= R * sin(EL0) * 0.55:
				return {"has": true, "point": p}
		return {"has": false, "point": Vector3.ZERO}

	func cell_at(point: Vector3) -> Dictionary:
		var r := point.length()
		if r < 0.001:
			return {"has": false}
		var el := asin(clampf(point.y / r, -1.0, 1.0))
		var step_el := (EL1 - EL0) / maxf(1.0, float(levels - 1))
		if el < EL0 - step_el * 0.6 or el > EL1 + step_el * 0.6:
			return {"has": false}
		var lv := clampi(int(round((el - EL0) / step_el)), 0, levels - 1)
		var st := posmod(int(round(atan2(point.z, point.x) / TAU * float(steps))), steps)
		return {"has": true, "ring": lv, "slot": st}

	func sweep_kind() -> String:
		return "arc"

	func sweep_xform(t: float) -> Transform3D:
		return Transform3D(Basis(Vector3.UP, -t * TAU), Vector3.ZERO)

	func sweep_span() -> Vector3:
		# x = radio, y = elevación mínima, z = elevación máxima.
		return Vector3(R, EL0 - 0.10, EL1 + 0.12)

	func camera_target() -> Vector3:
		return Vector3(0.0, R * 0.62, 0.0)

	func camera_defaults() -> Dictionary:
		## La cámara va DENTRO de la cúpula, mirando hacia arriba.
		##
		## Desde fuera, la mitad cercana ocluye a la lejana y más de la mitad de
		## las celdas dejan de ser alcanzables con un toque (medido: 25 de 80).
		## Desde dentro, cada rayo corta la esfera exactamente una vez: no hay
		## oclusión posible y todas las celdas son alcanzables orbitando. Y de
		## paso es lo que de verdad significa mirar el cielo.
		##
		## `dist_max` se limita por debajo del radio: si la cámara sale de la
		## esfera vuelve el problema de oclusión.
		return {"yaw": 0.9, "pitch": 0.62, "dist": 6.0,
				"pitch_min": 0.16, "pitch_max": 1.05,
				"dist_min": 2.2, "dist_max": R * 0.82}

	func build_base(parent: Node3D, hex: String) -> Array[Node3D]:
		var out: Array[Node3D] = []
		var mat := StandardMaterial3D.new()
		mat.albedo_color = Color(hex)
		mat.roughness = 1.0
		# Suelo-horizonte: sin él las estrellas flotan sin escala.
		var disc := CylinderMesh.new()
		disc.top_radius = R * 1.02
		disc.bottom_radius = R * 0.88
		disc.height = 0.5
		disc.radial_segments = 64
		var mi := MeshInstance3D.new()
		mi.mesh = disc
		mi.material_override = mat
		mi.position.y = -0.25
		mi.set_meta("ground_mat", mat)
		parent.add_child(mi)
		out.append(mi)

		# Paralelos: un anillo por nivel, a su altura y con su radio.
		for i in levels:
			var el := _el(i)
			var ring := TorusMesh.new()
			ring.inner_radius = R * cos(el) - 0.014
			ring.outer_radius = R * cos(el) + 0.014
			ring.rings = 64
			ring.ring_segments = 4
			out.append(Layouts._grid_mesh(parent, ring, Vector3(0, R * sin(el), 0)))
		return out


static func make(kind: String, levels: int, steps: int) -> Base:
	match kind:
		"wall": return Wall.new(levels, steps)
		"field": return Field.new(levels, steps)
		"dome": return Dome.new(levels, steps)
		_: return Radial.new(levels, steps)
