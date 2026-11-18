class_name MeshLib
extends RefCounted

## Formas procedurales de los objetos.
##
## Todo se compone con primitivas: no hay assets externos. Para el prototipo lo
## que importa es la SILUETA y la MASA — la masa es lo que el mapeo usa como
## registro, así que un objeto grave tiene que verse pesado a primera vista.
##
## `build()` devuelve las mallas más los materiales, para que quien la llame
## pueda animar el destello sin volver a recorrer el árbol.

class Built extends RefCounted:
	var root: Node3D
	var materials: Array[StandardMaterial3D] = []
	var emissions: Array[float] = []
	var light: OmniLight3D


static func build(shape: String, hex: String) -> Built:
	var b := Built.new()
	b.root = Node3D.new()
	match shape:
		"pine": _pine(b, hex)
		"bush": _bush(b, hex)
		"fern": _fern(b, hex)
		"flower": _flower(b, hex)
		"mushroom": _mushroom(b, hex)
		"rock": _rock(b, hex)
		"firefly": _firefly(b, hex)
		"coral": _coral(b, hex)
		"kelp": _kelp(b, hex)
		"fish": _fish(b, hex)
		"bubble": _bubble(b, hex)
		"anemone": _anemone(b, hex)
		"jelly": _jelly(b, hex)
		"tower": _tower(b, hex)
		"house": _house(b, hex)
		"antenna": _antenna(b, hex)
		"lamp": _lamp(b, hex)
		"ceiba": _ceiba(b, hex)
		"bamboo": _bamboo(b, hex)
		"liana": _liana(b, hex)
		"orchid": _orchid(b, hex)
		"bird": _bird(b, hex)
		"frog": _frog(b, hex)
		"lighthouse": _lighthouse(b, hex)
		"buoy": _buoy(b, hex)
		"wave": _wave(b, hex)
		"boat": _boat(b, hex)
		"gull": _gull(b, hex)
		"cloud": _cloud(b, hex)
		"moon": _moon(b, hex)
		"star": _star(b, hex)
		"comet": _comet(b, hex)
		"meteor": _meteor(b, hex)
		"floorlamp": _floorlamp(b, hex)
		"planter": _planter(b, hex)
		"book": _book(b, hex)
		"clock": _clock(b, hex)
		"candle": _candle(b, hex)
		"radio": _radio(b, hex)
		_: _add(b, _sphere(0.3), _mat(b, hex), Vector3(0, 0.3, 0))
	return b


# --------------------------------------------------------------------------
# helpers
# --------------------------------------------------------------------------

static func _mat(b: Built, hex: String, emission := 0.0, rough := 0.85,
		lighten := 0.0, alpha := 1.0) -> StandardMaterial3D:
	var col := Color(hex)
	if lighten != 0.0:
		col = col.lightened(lighten)
	col.a = alpha
	var m := StandardMaterial3D.new()
	m.albedo_color = col
	m.roughness = rough
	# La emisión se habilita siempre (aunque valga 0) para que el destello del
	# disparo tenga a dónde subir sin recrear el material.
	m.emission_enabled = true
	m.emission = col
	m.emission_energy_multiplier = emission
	if alpha < 1.0:
		m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	b.materials.append(m)
	b.emissions.append(emission)
	return m


static func _add(b: Built, mesh: Mesh, mat: StandardMaterial3D, pos: Vector3,
		rot := Vector3.ZERO, scl := Vector3.ONE) -> MeshInstance3D:
	var mi := MeshInstance3D.new()
	mi.mesh = mesh
	mi.material_override = mat
	mi.position = pos
	mi.rotation = rot
	mi.scale = scl
	b.root.add_child(mi)
	return mi


static func _cone(bottom: float, height: float, seg := 10) -> CylinderMesh:
	return _cyl(0.0, bottom, height, seg)


static func _cyl(top: float, bottom: float, height: float, seg := 8) -> CylinderMesh:
	var c := CylinderMesh.new()
	c.top_radius = top
	c.bottom_radius = bottom
	c.height = height
	c.radial_segments = seg
	c.rings = 1
	return c


static func _sphere(r: float, seg := 12, rings := 6, hemi := false) -> SphereMesh:
	var s := SphereMesh.new()
	s.radius = r
	s.height = r * (1.0 if hemi else 2.0)
	s.radial_segments = seg
	s.rings = rings
	s.is_hemisphere = hemi
	return s


static func _box(size: Vector3) -> BoxMesh:
	var m := BoxMesh.new()
	m.size = size
	return m


# --------------------------------------------------------------------------
# bosque
# --------------------------------------------------------------------------

static func _pine(b: Built, hex: String) -> void:
	var bark := _mat(b, "4a3b30")
	var leaf := _mat(b, hex)
	_add(b, _cyl(0.07, 0.10, 0.55), bark, Vector3(0, 0.275, 0))
	var y := 0.52
	for i in 3:
		var r := 0.42 - i * 0.10
		var h := 0.52 - i * 0.07
		_add(b, _cone(r, h), leaf, Vector3(0, y + h * 0.5 - 0.1, 0))
		y += h * 0.52


static func _bush(b: Built, hex: String) -> void:
	var leaf := _mat(b, hex)
	var top := _mat(b, hex, 0.0, 0.85, 0.18)
	_add(b, _sphere(0.30), leaf, Vector3(0, 0.28, 0))
	_add(b, _sphere(0.23), leaf, Vector3(-0.22, 0.20, 0.08))
	_add(b, _sphere(0.21), leaf, Vector3(0.20, 0.19, -0.10))
	_add(b, _sphere(0.17), top, Vector3(0.02, 0.46, 0.02))


static func _fern(b: Built, hex: String) -> void:
	var frond := _mat(b, hex)
	for i in 5:
		var a := (float(i) - 2.0) * 0.42
		_add(b, _box(Vector3(0.045, 0.62, 0.11)), frond,
			Vector3(sin(a) * 0.16, 0.30, cos(a) * 0.05),
			Vector3(0.0, a, sin(a) * 0.55))


static func _flower(b: Built, hex: String) -> void:
	var stem := _mat(b, "6f9464")
	var petal := _mat(b, hex, 0.15)
	var heart := _mat(b, "fff3c4", 0.6)
	_add(b, _cyl(0.022, 0.030, 0.52), stem, Vector3(0, 0.26, 0))
	for i in 5:
		var a := float(i) / 5.0 * TAU
		_add(b, _sphere(0.115, 8, 4), petal,
			Vector3(cos(a) * 0.12, 0.56, sin(a) * 0.12),
			Vector3.ZERO, Vector3(1.0, 0.45, 1.0))
	_add(b, _sphere(0.075, 8, 4), heart, Vector3(0, 0.585, 0))


static func _mushroom(b: Built, hex: String) -> void:
	var stem := _mat(b, "e8dcc8")
	var cap := _mat(b, hex, 0.1)
	_add(b, _cyl(0.075, 0.095, 0.34), stem, Vector3(0, 0.17, 0))
	_add(b, _sphere(0.29, 12, 6, true), cap, Vector3(0, 0.32, 0),
		Vector3.ZERO, Vector3(1.0, 0.8, 1.0))
	_add(b, _sphere(0.05, 6, 3), stem, Vector3(-0.11, 0.42, 0.06))


static func _rock(b: Built, hex: String) -> void:
	# Esfera de pocos segmentos escalada de forma irregular: lee como roca
	# facetada sin necesidad de modelarla.
	_add(b, _sphere(0.30, 6, 3), _mat(b, hex, 0.0, 0.65), Vector3(0, 0.15, 0),
		Vector3(0.3, 0.7, 0.15), Vector3(1.25, 0.62, 0.95))


static func _firefly(b: Built, hex: String) -> void:
	_add(b, _sphere(0.085, 10, 5), _mat(b, hex, 2.2), Vector3(0, 0.72, 0))
	b.light = OmniLight3D.new()
	b.light.light_color = Color(hex)
	b.light.light_energy = 0.9
	b.light.omni_range = 3.4
	b.light.position = Vector3(0, 0.72, 0)
	b.root.add_child(b.light)


# --------------------------------------------------------------------------
# acuario / mar
# --------------------------------------------------------------------------

static func _coral(b: Built, hex: String) -> void:
	var body := _mat(b, hex, 0.08)
	var tip := _mat(b, hex, 0.25, 0.7, 0.25)
	_add(b, _cyl(0.09, 0.14, 0.34), body, Vector3(0, 0.17, 0))
	# Ramas en abanico: lo que distingue un coral de un arbusto es que las
	# ramas suben divergiendo, no que la copa sea redonda.
	for i in 4:
		var a := float(i) / 4.0 * TAU + 0.4
		var lean := 0.55
		_add(b, _cyl(0.03, 0.07, 0.46), body,
			Vector3(cos(a) * 0.17, 0.50, sin(a) * 0.17),
			Vector3(sin(a) * lean, 0.0, -cos(a) * lean))
		_add(b, _sphere(0.07, 8, 4), tip,
			Vector3(cos(a) * 0.32, 0.70, sin(a) * 0.32))


static func _kelp(b: Built, hex: String) -> void:
	var leaf := _mat(b, hex, 0.05)
	# Cinta que se curva: tres segmentos con giro progresivo.
	for i in 3:
		var t := float(i)
		_add(b, _box(Vector3(0.22, 0.42, 0.03)), leaf,
			Vector3(sin(t * 1.1) * 0.10, 0.22 + t * 0.40, cos(t * 0.7) * 0.06),
			Vector3(0.0, t * 0.5, sin(t * 1.3) * 0.30))


static func _fish(b: Built, hex: String) -> void:
	var body := _mat(b, hex, 0.12, 0.45)
	var fin := _mat(b, hex, 0.2, 0.5, 0.3, 0.75)
	_add(b, _sphere(0.20, 10, 6), body, Vector3(0, 0.0, 0),
		Vector3.ZERO, Vector3(1.5, 0.85, 0.55))
	_add(b, _cone(0.16, 0.24), fin, Vector3(-0.32, 0.0, 0.0), Vector3(0, 0, PI * 0.5))
	_add(b, _sphere(0.045, 6, 3), _mat(b, "1a1a22"), Vector3(0.20, 0.05, 0.09))


static func _bubble(b: Built, hex: String) -> void:
	# Semitransparente y emisiva: una burbuja opaca parece una canica.
	_add(b, _sphere(0.19, 12, 7), _mat(b, hex, 0.5, 0.05, 0.35, 0.42), Vector3.ZERO)
	_add(b, _sphere(0.05, 6, 3), _mat(b, "ffffff", 0.9), Vector3(-0.07, 0.08, 0.10))


static func _anemone(b: Built, hex: String) -> void:
	var base := _mat(b, hex, 0.1)
	var arm := _mat(b, hex, 0.45, 0.5, 0.3)
	_add(b, _sphere(0.19, 10, 5, true), base, Vector3(0, 0.05, 0))
	for i in 9:
		var a := float(i) / 9.0 * TAU
		var lean := 0.75
		_add(b, _cyl(0.012, 0.030, 0.40), arm,
			Vector3(cos(a) * 0.13, 0.24, sin(a) * 0.13),
			Vector3(sin(a) * lean, 0.0, -cos(a) * lean))


static func _jelly(b: Built, hex: String) -> void:
	var bell := _mat(b, hex, 0.55, 0.15, 0.0, 0.5)
	var tent := _mat(b, hex, 0.35, 0.3, 0.3, 0.6)
	_add(b, _sphere(0.30, 14, 7, true), bell, Vector3(0, 0.30, 0),
		Vector3.ZERO, Vector3(1.0, 1.25, 1.0))
	for i in 6:
		var a := float(i) / 6.0 * TAU
		_add(b, _box(Vector3(0.028, 0.52, 0.028)), tent,
			Vector3(cos(a) * 0.16, 0.00, sin(a) * 0.16))
	b.light = OmniLight3D.new()
	b.light.light_color = Color(hex)
	b.light.light_energy = 0.7
	b.light.omni_range = 3.0
	b.light.position = Vector3(0, 0.3, 0)
	b.root.add_child(b.light)


# --------------------------------------------------------------------------
# ciudad / interior
# --------------------------------------------------------------------------

static func _tower(b: Built, hex: String) -> void:
	var wall := _mat(b, hex, 0.0, 0.6)
	# Ventanas encendidas: emisivas, para que el glow las convierta en luz.
	var win := _mat(b, "ffd9a0", 1.6, 0.2)
	_add(b, _box(Vector3(0.46, 1.30, 0.46)), wall, Vector3(0, 0.65, 0))
	_add(b, _box(Vector3(0.34, 0.22, 0.34)), wall, Vector3(0, 1.40, 0))
	for i in 4:
		var y := 0.28 + i * 0.28
		_add(b, _box(Vector3(0.48, 0.09, 0.10)), win, Vector3(0, y, 0.20))
		_add(b, _box(Vector3(0.10, 0.09, 0.48)), win, Vector3(0.20, y, 0))


static func _house(b: Built, hex: String) -> void:
	var wall := _mat(b, hex, 0.0, 0.7)
	var roof := _mat(b, hex, 0.0, 0.7, -0.25)
	var win := _mat(b, "ffd9a0", 1.4, 0.2)
	_add(b, _box(Vector3(0.54, 0.42, 0.48)), wall, Vector3(0, 0.21, 0))
	_add(b, _cone(0.42, 0.34, 4), roof, Vector3(0, 0.59, 0), Vector3(0, PI * 0.25, 0))
	_add(b, _box(Vector3(0.14, 0.14, 0.06)), win, Vector3(0, 0.24, 0.25))


static func _antenna(b: Built, hex: String) -> void:
	var mast := _mat(b, hex, 0.0, 0.35)
	var tip := _mat(b, "ff7a7a", 2.0, 0.2)
	_add(b, _box(Vector3(0.16, 0.14, 0.16)), mast, Vector3(0, 0.07, 0))
	_add(b, _cyl(0.015, 0.05, 1.05, 6), mast, Vector3(0, 0.60, 0))
	for i in 3:
		var y := 0.34 + i * 0.26
		var w := 0.30 - i * 0.07
		_add(b, _box(Vector3(w, 0.020, 0.020)), mast, Vector3(0, y, 0))
	_add(b, _sphere(0.05, 8, 4), tip, Vector3(0, 1.16, 0))


static func _lamp(b: Built, hex: String) -> void:
	var post := _mat(b, hex, 0.0, 0.4)
	var bulb := _mat(b, "ffe0a8", 2.4, 0.2)
	_add(b, _cyl(0.026, 0.042, 0.86, 6), post, Vector3(0, 0.43, 0))
	_add(b, _box(Vector3(0.20, 0.030, 0.030)), post, Vector3(0.09, 0.86, 0))
	_add(b, _sphere(0.075, 8, 4), bulb, Vector3(0.18, 0.83, 0))
	b.light = OmniLight3D.new()
	b.light.light_color = Color("ffe0a8")
	b.light.light_energy = 1.1
	b.light.omni_range = 3.2
	b.light.position = Vector3(0.18, 0.83, 0)
	b.root.add_child(b.light)


# --------------------------------------------------------------------------
# selva
# --------------------------------------------------------------------------

static func _ceiba(b: Built, hex: String) -> void:
	## Árbol de dosel: tronco alto y copa ancha y plana. Lo contrario del pino —
	## en la selva la luz se pelea arriba, no abajo.
	var bark := _mat(b, "5b4432")
	var leaf := _mat(b, hex)
	_add(b, _cyl(0.10, 0.20, 1.10), bark, Vector3(0, 0.55, 0))
	for i in 3:
		var a := float(i) / 3.0 * TAU
		_add(b, _cyl(0.05, 0.09, 0.42), bark,
			Vector3(cos(a) * 0.16, 0.95, sin(a) * 0.16),
			Vector3(sin(a) * 0.6, 0.0, -cos(a) * 0.6))
	_add(b, _sphere(0.62, 12, 6), leaf, Vector3(0, 1.28, 0),
		Vector3.ZERO, Vector3(1.0, 0.42, 1.0))
	_add(b, _sphere(0.40, 10, 5), leaf, Vector3(-0.34, 1.14, 0.16),
		Vector3.ZERO, Vector3(1.0, 0.45, 1.0))
	_add(b, _sphere(0.34, 10, 5), leaf, Vector3(0.32, 1.18, -0.20),
		Vector3.ZERO, Vector3(1.0, 0.45, 1.0))


static func _bamboo(b: Built, hex: String) -> void:
	var cane := _mat(b, hex)
	var node_mat := _mat(b, hex, 0.0, 0.8, -0.2)
	for c in 3:
		var x := (float(c) - 1.0) * 0.16
		var lean := (float(c) - 1.0) * 0.07
		for seg in 4:
			var y := 0.22 + seg * 0.40
			_add(b, _cyl(0.055, 0.06, 0.36, 6), cane,
				Vector3(x + lean * seg, y, lean * seg * 0.4), Vector3(0, 0, lean))
			_add(b, _cyl(0.068, 0.068, 0.035, 6), node_mat,
				Vector3(x + lean * seg, y + 0.19, lean * seg * 0.4))


static func _liana(b: Built, hex: String) -> void:
	## Colgante: la masa está ARRIBA y cae. Es la silueta opuesta a una planta.
	var vine := _mat(b, hex)
	var leaf := _mat(b, hex, 0.0, 0.8, 0.22)
	for i in 3:
		var a := float(i) / 3.0 * TAU
		var x := cos(a) * 0.13
		var z := sin(a) * 0.13
		for seg in 4:
			_add(b, _cyl(0.022, 0.026, 0.30, 5), vine,
				Vector3(x + sin(seg * 1.4) * 0.05, 1.15 - seg * 0.28, z),
				Vector3(0, 0, sin(seg * 1.4) * 0.25))
		_add(b, _sphere(0.10, 8, 4), leaf, Vector3(x, 0.10, z),
			Vector3.ZERO, Vector3(1.4, 0.4, 1.0))


static func _orchid(b: Built, hex: String) -> void:
	var stem := _mat(b, "6f9464")
	var petal := _mat(b, hex, 0.25)
	var lip := _mat(b, "fff3c4", 0.5)
	_add(b, _cyl(0.020, 0.028, 0.46), stem, Vector3(0, 0.23, 0))
	for i in 3:
		var y := 0.30 + i * 0.16
		var a := float(i) * 2.1
		for p in 3:
			var pa := a + float(p) / 3.0 * TAU
			_add(b, _sphere(0.085, 6, 3), petal,
				Vector3(cos(pa) * 0.10, y, sin(pa) * 0.10),
				Vector3.ZERO, Vector3(1.3, 0.35, 0.9))
		_add(b, _sphere(0.05, 6, 3), lip, Vector3(0, y, 0))


static func _bird(b: Built, hex: String) -> void:
	var body := _mat(b, hex, 0.15)
	var wing := _mat(b, hex, 0.25, 0.6, 0.25)
	var beak := _mat(b, "ffb347", 0.4)
	_add(b, _sphere(0.16, 10, 5), body, Vector3(0, 0.60, 0),
		Vector3.ZERO, Vector3(1.3, 0.9, 0.8))
	_add(b, _sphere(0.10, 8, 4), body, Vector3(0.17, 0.72, 0))
	_add(b, _cone(0.05, 0.14), beak, Vector3(0.29, 0.72, 0), Vector3(0, 0, -PI * 0.5))
	for sgn in [-1.0, 1.0]:
		_add(b, _box(Vector3(0.20, 0.02, 0.11)), wing,
			Vector3(-0.03, 0.66, sgn * 0.13), Vector3(sgn * 0.5, 0, 0))
	_add(b, _box(Vector3(0.22, 0.02, 0.08)), wing, Vector3(-0.22, 0.62, 0))


static func _frog(b: Built, hex: String) -> void:
	var skin := _mat(b, hex, 0.0, 0.4)
	var eye := _mat(b, "fff3c4", 0.5)
	_add(b, _sphere(0.22, 10, 5), skin, Vector3(0, 0.16, 0),
		Vector3.ZERO, Vector3(1.2, 0.75, 1.0))
	for sgn in [-1.0, 1.0]:
		_add(b, _sphere(0.07, 6, 3), eye, Vector3(0.13, 0.28, sgn * 0.09))
		_add(b, _cyl(0.035, 0.045, 0.18, 5), skin,
			Vector3(-0.10, 0.10, sgn * 0.18), Vector3(sgn * 0.6, 0, 0.4))


# --------------------------------------------------------------------------
# mar
# --------------------------------------------------------------------------

static func _lighthouse(b: Built, hex: String) -> void:
	var wall := _mat(b, hex, 0.0, 0.6)
	var band := _mat(b, "c94f4f", 0.0, 0.6)
	var lamp := _mat(b, "ffe9a8", 2.6, 0.2)
	_add(b, _cyl(0.16, 0.30, 1.20, 10), wall, Vector3(0, 0.60, 0))
	for i in 2:
		_add(b, _cyl(0.24 - i * 0.04, 0.26 - i * 0.04, 0.16, 10), band,
			Vector3(0, 0.42 + i * 0.44, 0))
	_add(b, _cyl(0.20, 0.20, 0.22, 10), lamp, Vector3(0, 1.31, 0))
	_add(b, _cone(0.24, 0.20, 10), wall, Vector3(0, 1.50, 0))
	b.light = OmniLight3D.new()
	b.light.light_color = Color("ffe9a8")
	b.light.light_energy = 1.4
	b.light.omni_range = 5.0
	b.light.position = Vector3(0, 1.31, 0)
	b.root.add_child(b.light)


static func _buoy(b: Built, hex: String) -> void:
	var hull := _mat(b, hex, 0.1, 0.5)
	var bell := _mat(b, "d8d2c0", 0.3, 0.35)
	_add(b, _sphere(0.26, 10, 5), hull, Vector3(0, 0.20, 0),
		Vector3.ZERO, Vector3(1.0, 0.85, 1.0))
	_add(b, _cyl(0.06, 0.10, 0.38, 6), hull, Vector3(0, 0.50, 0))
	_add(b, _sphere(0.13, 8, 4, true), bell, Vector3(0, 0.66, 0),
		Vector3(PI, 0, 0))


static func _wave(b: Built, hex: String) -> void:
	## Cresta: tres arcos escalonados y translúcidos. No es un objeto sólido, y
	## por eso lleva alfa: una ola opaca parece una duna.
	for i in 3:
		var m := _mat(b, hex, 0.30, 0.15, float(i) * 0.15, 0.55)
		_add(b, _sphere(0.34 - i * 0.07, 12, 4, true), m,
			Vector3(-0.10 + i * 0.12, 0.05 + i * 0.09, 0.0),
			Vector3(0, 0, -0.25), Vector3(1.5, 0.6, 0.75))


static func _boat(b: Built, hex: String) -> void:
	var hull := _mat(b, hex, 0.0, 0.55)
	var sail := _mat(b, "efe6d2", 0.15, 0.7)
	_add(b, _sphere(0.34, 10, 4, true), hull, Vector3(0, 0.14, 0),
		Vector3(PI, 0, 0), Vector3(1.5, 0.5, 0.8))
	_add(b, _cyl(0.020, 0.026, 0.80, 5), hull, Vector3(0, 0.52, 0))
	_add(b, _box(Vector3(0.02, 0.52, 0.34)), sail, Vector3(0.02, 0.60, 0.16))


static func _gull(b: Built, hex: String) -> void:
	var body := _mat(b, hex, 0.2, 0.5)
	## Alas en V muy abierta: a distancia una gaviota ES esa silueta.
	_add(b, _sphere(0.10, 8, 4), body, Vector3(0, 0.85, 0),
		Vector3.ZERO, Vector3(1.5, 0.8, 0.7))
	for sgn in [-1.0, 1.0]:
		_add(b, _box(Vector3(0.10, 0.020, 0.40)), body,
			Vector3(0, 0.89, sgn * 0.24), Vector3(sgn * 0.42, 0, 0))


# --------------------------------------------------------------------------
# cielo
# --------------------------------------------------------------------------

static func _cloud(b: Built, hex: String) -> void:
	var puff := _mat(b, hex, 0.12, 0.9, 0.0, 0.72)
	for p in [Vector3(0, 0, 0), Vector3(-0.30, -0.06, 0.08),
			Vector3(0.28, -0.04, -0.06), Vector3(-0.05, 0.16, -0.10)]:
		_add(b, _sphere(0.30 - absf(p.x) * 0.25, 10, 5), puff, p,
			Vector3.ZERO, Vector3(1.25, 0.7, 1.0))


static func _moon(b: Built, hex: String) -> void:
	var rock := _mat(b, hex, 0.35, 0.95)
	var crater := _mat(b, hex, 0.10, 1.0, -0.22)
	_add(b, _sphere(0.42, 16, 9), rock, Vector3.ZERO)
	for p in [Vector3(0.16, 0.14, 0.34), Vector3(-0.20, -0.06, 0.34),
			Vector3(0.02, -0.24, 0.32)]:
		_add(b, _sphere(0.09, 8, 4), crater, p, Vector3.ZERO, Vector3(1, 1, 0.35))
	b.light = OmniLight3D.new()
	b.light.light_color = Color(hex)
	b.light.light_energy = 1.0
	b.light.omni_range = 6.0
	b.root.add_child(b.light)


static func _star(b: Built, hex: String) -> void:
	## Cuatro púas cruzadas: el destello de una estrella se lee mejor que su
	## disco, que a esta escala sería un punto.
	var core := _mat(b, hex, 2.6, 0.1)
	_add(b, _sphere(0.10, 10, 5), core, Vector3.ZERO)
	for rot in [Vector3.ZERO, Vector3(0, 0, PI * 0.5), Vector3(PI * 0.5, 0, 0)]:
		_add(b, _box(Vector3(0.035, 0.46, 0.035)), core, Vector3.ZERO, rot)
	b.light = OmniLight3D.new()
	b.light.light_color = Color(hex)
	b.light.light_energy = 0.8
	b.light.omni_range = 3.0
	b.root.add_child(b.light)


static func _comet(b: Built, hex: String) -> void:
	var head := _mat(b, hex, 2.8, 0.1)
	_add(b, _sphere(0.13, 10, 5), head, Vector3.ZERO)
	# Cola que se afina y se apaga: cada segmento con su propio material.
	for i in 5:
		var tail := _mat(b, hex, 1.8 - i * 0.32, 0.2, 0.1, 0.65 - i * 0.11)
		_add(b, _sphere(0.11 - i * 0.017, 8, 4), tail,
			Vector3(-0.16 - i * 0.17, -0.03 * i, 0.0),
			Vector3.ZERO, Vector3(1.6, 0.8, 0.8))
	b.light = OmniLight3D.new()
	b.light.light_color = Color(hex)
	b.light.light_energy = 1.1
	b.light.omni_range = 3.6
	b.root.add_child(b.light)


static func _meteor(b: Built, hex: String) -> void:
	var rock := _mat(b, hex, 0.0, 0.75)
	var hot := _mat(b, "ff9a5a", 1.8, 0.3)
	_add(b, _sphere(0.20, 6, 3), rock, Vector3.ZERO,
		Vector3(0.4, 0.8, 0.2), Vector3(1.2, 0.85, 1.0))
	_add(b, _sphere(0.09, 6, 3), hot, Vector3(0.16, -0.05, 0.05))


# --------------------------------------------------------------------------
# interior
# --------------------------------------------------------------------------

static func _floorlamp(b: Built, hex: String) -> void:
	var post := _mat(b, hex, 0.0, 0.4)
	var shade := _mat(b, "ffe0b0", 1.5, 0.6)
	_add(b, _cyl(0.20, 0.24, 0.05, 12), post, Vector3(0, 0.02, 0))
	_add(b, _cyl(0.020, 0.026, 0.95, 6), post, Vector3(0, 0.50, 0))
	_add(b, _cyl(0.16, 0.26, 0.32, 12), shade, Vector3(0, 1.10, 0))
	b.light = OmniLight3D.new()
	b.light.light_color = Color("ffe0b0")
	b.light.light_energy = 1.3
	b.light.omni_range = 4.0
	b.light.position = Vector3(0, 1.05, 0)
	b.root.add_child(b.light)


static func _planter(b: Built, hex: String) -> void:
	var pot := _mat(b, "b06a4a", 0.0, 0.8)
	var leaf := _mat(b, hex)
	_add(b, _cyl(0.24, 0.17, 0.30, 10), pot, Vector3(0, 0.15, 0))
	for i in 5:
		var a := float(i) / 5.0 * TAU
		_add(b, _box(Vector3(0.16, 0.44, 0.03)), leaf,
			Vector3(cos(a) * 0.09, 0.48, sin(a) * 0.09),
			Vector3(sin(a) * 0.45, a, -cos(a) * 0.45))


static func _book(b: Built, hex: String) -> void:
	## Pila de tres: un libro suelto no se lee a esta escala.
	for i in 3:
		var cover := _mat(b, hex, 0.0, 0.75, float(i) * 0.16)
		_add(b, _box(Vector3(0.40 - i * 0.04, 0.10, 0.30 - i * 0.03)), cover,
			Vector3(i * 0.02, 0.06 + i * 0.11, i * 0.015),
			Vector3(0, float(i) * 0.18, 0))


static func _clock(b: Built, hex: String) -> void:
	var case_mat := _mat(b, hex, 0.0, 0.5)
	var face := _mat(b, "f2ecd8", 0.5, 0.3)
	var hand := _mat(b, "2b2b33")
	_add(b, _cyl(0.30, 0.30, 0.10, 16), case_mat, Vector3(0, 0.30, 0),
		Vector3(PI * 0.5, 0, 0))
	_add(b, _cyl(0.25, 0.25, 0.03, 16), face, Vector3(0, 0.30, 0.06),
		Vector3(PI * 0.5, 0, 0))
	_add(b, _box(Vector3(0.02, 0.17, 0.01)), hand, Vector3(0, 0.37, 0.09))
	_add(b, _box(Vector3(0.12, 0.02, 0.01)), hand, Vector3(0.06, 0.30, 0.09))


static func _candle(b: Built, hex: String) -> void:
	var wax := _mat(b, hex, 0.15, 0.6)
	var flame := _mat(b, "ffd27a", 3.0, 0.1)
	_add(b, _cyl(0.09, 0.11, 0.46, 10), wax, Vector3(0, 0.23, 0))
	_add(b, _sphere(0.055, 8, 5), flame, Vector3(0, 0.54, 0),
		Vector3.ZERO, Vector3(0.7, 1.7, 0.7))
	b.light = OmniLight3D.new()
	b.light.light_color = Color("ffd27a")
	b.light.light_energy = 1.2
	b.light.omni_range = 3.0
	b.light.position = Vector3(0, 0.54, 0)
	b.root.add_child(b.light)


static func _radio(b: Built, hex: String) -> void:
	var body := _mat(b, hex, 0.0, 0.55)
	var mesh_mat := _mat(b, hex, 0.0, 0.9, -0.28)
	var dial := _mat(b, "ffd9a0", 1.2, 0.3)
	_add(b, _box(Vector3(0.52, 0.34, 0.24)), body, Vector3(0, 0.17, 0))
	_add(b, _cyl(0.11, 0.11, 0.03, 12), mesh_mat, Vector3(-0.12, 0.19, 0.13),
		Vector3(PI * 0.5, 0, 0))
	_add(b, _box(Vector3(0.16, 0.05, 0.02)), dial, Vector3(0.13, 0.22, 0.13))
	_add(b, _cyl(0.010, 0.014, 0.42, 5), body, Vector3(0.20, 0.52, -0.06),
		Vector3(0, 0, 0.28))
