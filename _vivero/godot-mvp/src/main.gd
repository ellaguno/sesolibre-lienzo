extends Node3D

## Punto de entrada. Crea el árbol de escena por código, conecta señales y
## resuelve la entrada (toque vs arrastre).

const GARDEN_DIR := "user://jardines/"
const POSTCARD_DIR := "user://postales/"
const PAYLOAD_DIR := "user://payloads/"

## Un gesto cuenta como toque, no como órbita, si se movió poco y duró poco.
const TAP_MAX_PX := 14.0
const TAP_MAX_SEC := 0.45

var garden: Garden
var diorama: Diorama
var cam: OrbitCamera
var ui: MainUI

var tool_id := "arbol"
var appreciation := false

var _press_pos := Vector2.ZERO
var _press_usec := 0
var _dragging := false
var _drag_moved := 0.0
var _touches: Dictionary = {}          ## index -> Vector2 (para el pinch)
var _pinch_ref := -1.0
var _rendered_player: AudioStreamPlayer


func _ready() -> void:
	for d in [GARDEN_DIR, POSTCARD_DIR, PAYLOAD_DIR]:
		DirAccess.make_dir_recursive_absolute(d)

	garden = Garden.new()
	garden.seed_default()

	cam = OrbitCamera.new()
	add_child(cam)

	diorama = Diorama.new()
	add_child(diorama)
	diorama.setup(garden)
	diorama.cell_tapped.connect(_on_cell_tapped)

	ui = MainUI.new()
	add_child(ui)
	_wire_ui()

	garden.changed.connect(_on_garden_changed)
	Conductor.cycle_started.connect(func(): SoundBank.pulse(garden.mood_key))

	_apply_biome()
	ui.set_tool(tool_id)
	_on_garden_changed()

	Conductor.bpm = garden.bpm
	Conductor.play()
	ui.set_playing(true)


func _wire_ui() -> void:
	ui.tool_selected.connect(func(id: String):
		tool_id = id
		ui.set_tool(id))
	ui.transport_toggled.connect(func():
		Conductor.toggle()
		ui.set_playing(Conductor.playing))
	ui.seed_requested.connect(func(): garden.seed_default())
	ui.clear_requested.connect(func(): garden.clear())
	ui.mood_changed.connect(func(key: String):
		garden.set_mood(key)
		_apply_mood(key))
	ui.biome_changed.connect(func(id: String):
		garden.set_biome(id)
		_apply_biome()
		garden.seed_default())
	ui.transpose_changed.connect(func(v: int):
		garden.set_transpose(v)
		ui.set_stats(garden))
	ui.bpm_changed.connect(func(v: float):
		garden.bpm = v
		Conductor.bpm = v
		ui.set_stats(garden))
	ui.appreciation_toggled.connect(func(): _set_appreciation(not appreciation))
	ui.gallery_toggled.connect(func(): ui.toggle_gallery(_gallery_entries()))
	ui.postcard_requested.connect(_save_postcard)
	ui.payload_requested.connect(_save_payload)
	ui.save_requested.connect(_save_garden)
	ui.load_requested.connect(_load_garden)
	ui.ai_render_requested.connect(_request_ai_render)

	AIRender.progress.connect(func(estado: String, intento: int):
		ui.set_ai_busy(true, "%s %d" % [estado.substr(0, 8), intento]))
	AIRender.finished.connect(_on_ai_finished)
	AIRender.failed.connect(func(motivo: String):
		ui.set_ai_busy(false)
		ui.toast(motivo))


## Todo lo que depende del bioma: paleta, encuadre de cámara y herramienta
## activa. El diorama se reconstruye solo, por la señal `biome_changed`.
func _apply_biome() -> void:
	ui.set_biome(garden.biome_id)
	tool_id = String(garden.palette()[0])
	ui.set_tool(tool_id)
	cam.apply_layout(diorama.layout)
	Conductor.steps_per_cycle = garden.steps()
	Conductor.step_beats = Mapping.step_beats(garden.biome_id)
	_apply_mood(garden.mood_key)


func _apply_mood(key: String) -> void:
	ui.set_mood(key)
	diorama.set_mood(key)
	SoundBank.set_environment(garden.biome_id, key)
	garden.bpm = Mapping.bpm_for(garden.biome_id, key)
	Conductor.bpm = garden.bpm
	ui.set_stats(garden)


## La cámara es la posición de escucha. Se refresca en _process y no en cada
## gesto porque el suavizado de la cámara sigue moviéndose después de soltar.
func _process(_delta: float) -> void:
	SoundBank.set_perspective(cam.top_down_amount(), cam.distance_amount())


func _on_garden_changed() -> void:
	SoundBank.set_density(garden.count(), garden.levels() * garden.steps())
	ui.set_stats(garden)


func _on_cell_tapped(ring: int, slot: int, occupied: bool) -> void:
	if occupied:
		garden.erase_at(ring, slot)
		return
	if garden.place(ring, slot, tool_id):
		# El feedback inmediato es la mitad de la sensación de plantar: no se
		# espera a que el barrido llegue.
		diorama.preview(ring, slot)


func _set_appreciation(on: bool) -> void:
	appreciation = on
	ui.set_appreciation(on)
	diorama.set_appreciation(on)
	cam.frame_for_appreciation(on)


# --------------------------------------------------------------------------
# entrada
# --------------------------------------------------------------------------

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo:
		_handle_key(event as InputEventKey)
		return

	# --- ratón ---
	if event is InputEventMouseButton:
		var mb := event as InputEventMouseButton
		if mb.button_index == MOUSE_BUTTON_WHEEL_UP:
			cam.zoom(0.9)
		elif mb.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			cam.zoom(1.1)
		elif mb.button_index == MOUSE_BUTTON_LEFT:
			if mb.pressed:
				_begin_gesture(mb.position)
			else:
				_end_gesture(mb.position)
	elif event is InputEventMouseMotion and _dragging:
		var mm := event as InputEventMouseMotion
		_drag_moved += mm.relative.length()
		if _drag_moved > TAP_MAX_PX:
			cam.orbit(mm.relative)

	# --- táctil ---
	elif event is InputEventScreenTouch:
		var st := event as InputEventScreenTouch
		if st.pressed:
			_touches[st.index] = st.position
			if _touches.size() == 1:
				_begin_gesture(st.position)
			else:
				# Al entrar un segundo dedo el gesto deja de ser un toque.
				_dragging = false
				_pinch_ref = -1.0
		else:
			_touches.erase(st.index)
			if _touches.is_empty():
				_end_gesture(st.position)
			_pinch_ref = -1.0
	elif event is InputEventScreenDrag:
		var sd := event as InputEventScreenDrag
		_touches[sd.index] = sd.position
		if _touches.size() >= 2:
			_handle_pinch()
		elif _dragging:
			_drag_moved += sd.relative.length()
			if _drag_moved > TAP_MAX_PX:
				cam.orbit(sd.relative)
	elif event is InputEventMagnifyGesture:
		cam.zoom(1.0 / maxf(0.2, (event as InputEventMagnifyGesture).factor))


func _handle_pinch() -> void:
	var pts := _touches.values()
	var d := (pts[0] as Vector2).distance_to(pts[1] as Vector2)
	if _pinch_ref < 0.0:
		_pinch_ref = d
		return
	if d > 1.0 and _pinch_ref > 1.0:
		cam.zoom(_pinch_ref / d)
	_pinch_ref = d


func _begin_gesture(pos: Vector2) -> void:
	if ui.blocks_pointer(pos):
		return
	_dragging = true
	_drag_moved = 0.0
	_press_pos = pos
	_press_usec = Time.get_ticks_usec()


func _end_gesture(pos: Vector2) -> void:
	if not _dragging:
		return
	_dragging = false
	var secs := float(Time.get_ticks_usec() - _press_usec) / 1_000_000.0
	if _drag_moved > TAP_MAX_PX or secs > TAP_MAX_SEC:
		return
	# En apreciación se está mirando, no construyendo: un toque solo sale del modo.
	if appreciation:
		_set_appreciation(false)
		return
	var hit := diorama.pick(cam.project_ray_origin(pos), cam.project_ray_normal(pos))
	if bool(hit["has"]):
		diorama.handle_tap(hit["point"] as Vector3)


func _handle_key(key: InputEventKey) -> void:
	match key.keycode:
		KEY_SPACE:
			Conductor.toggle()
			ui.set_playing(Conductor.playing)
		KEY_A:
			_set_appreciation(not appreciation)
		KEY_C:
			garden.clear()
		KEY_S:
			_save_garden()
		KEY_P:
			_save_postcard()
		KEY_ESCAPE:
			if appreciation:
				_set_appreciation(false)
		_:
			# 1..7 seleccionan herramienta.
			var n := key.keycode - KEY_1
			var pal := garden.palette()
			if n >= 0 and n < pal.size():
				tool_id = String(pal[n])
				ui.set_tool(tool_id)


# --------------------------------------------------------------------------
# persistencia y exportación
# --------------------------------------------------------------------------

func _slug() -> String:
	var t := Time.get_datetime_dict_from_system()
	return "%04d%02d%02d-%02d%02d%02d" % [t.year, t.month, t.day, t.hour, t.minute, t.second]


## Ruta libre dentro de `dir`. El sello de tiempo tiene resolución de segundos,
## así que dos guardados seguidos colisionan y el segundo pisaba al primero sin
## avisar. Se añade un sufijo hasta encontrar un hueco.
func _unique_path(dir: String, prefix: String, ext: String) -> String:
	var stem := prefix + _slug()
	var path := dir + stem + ext
	var n := 2
	while FileAccess.file_exists(path) and n < 1000:
		path = "%s%s-%d%s" % [dir, stem, n, ext]
		n += 1
	return path


func _write_json(path: String, data: Dictionary) -> bool:
	var f := FileAccess.open(path, FileAccess.WRITE)
	if f == null:
		push_error("No se pudo escribir %s (error %d)" % [path, FileAccess.get_open_error()])
		return false
	f.store_string(JSON.stringify(data, "  "))
	f.close()
	return true


func _save_garden() -> void:
	if garden.count() == 0:
		ui.toast("El jardín está vacío.")
		return
	garden.title = "%s %s" % [String(Biomes.biome(garden.biome_id)["nm"]), _slug()]
	var path := _unique_path(GARDEN_DIR, "", ".json")
	if _write_json(path, garden.to_dict()):
		ui.toast("Guardado en la galería.")


func _load_garden(path: String) -> void:
	var f := FileAccess.open(path, FileAccess.READ)
	if f == null:
		ui.toast("No se pudo abrir el jardín.")
		return
	var parsed = JSON.parse_string(f.get_as_text())
	f.close()
	if typeof(parsed) != TYPE_DICTIONARY:
		ui.toast("Archivo de jardín corrupto.")
		return

	var loaded := Garden.from_dict(parsed as Dictionary)
	# Se conserva el objeto `garden` (todas las señales cuelgan de él) y se le
	# trasplanta el contenido, en vez de reconstruir el diorama entero.
	var biome_changed := loaded.biome_id != garden.biome_id
	garden.cells = loaded.cells
	garden.biome_id = loaded.biome_id
	garden.mood_key = loaded.mood_key
	garden.bpm = loaded.bpm
	garden.transpose = loaded.transpose
	garden.title = loaded.title
	if biome_changed:
		garden.biome_changed.emit()
	_apply_biome()
	garden.changed.emit()
	ui.toggle_gallery(_gallery_entries())
	ui.toast("Cargado: %s" % garden.title)


func _gallery_entries() -> Array[Dictionary]:
	var out: Array[Dictionary] = []
	var dir := DirAccess.open(GARDEN_DIR)
	if dir == null:
		return out
	var names := dir.get_files()
	names.sort()
	names.reverse()
	for name in names:
		if not name.ends_with(".json"):
			continue
		var f := FileAccess.open(GARDEN_DIR + name, FileAccess.READ)
		if f == null:
			continue
		var parsed = JSON.parse_string(f.get_as_text())
		f.close()
		if typeof(parsed) != TYPE_DICTIONARY:
			continue
		var d := parsed as Dictionary
		out.append({
			"path": GARDEN_DIR + name,
			"titulo": String(d.get("titulo", name)),
			"objetos": (d.get("celdas", []) as Array).size(),
		})
	return out


## Postal: captura del viewport en PNG. NO es el render cinematográfico de la
## fase 3 (ese usa la misma escena con más muestras, DOF y cámara orbital) ni un
## vídeo — ver README para la exportación de vídeo con Movie Maker.
func _save_postcard() -> void:
	var was_appreciation := appreciation
	if not was_appreciation:
		_set_appreciation(true)
	# Se espera a que la UI acabe de desvanecerse antes de capturar.
	await get_tree().create_timer(0.55).timeout
	await RenderingServer.frame_post_draw

	var img := get_viewport().get_texture().get_image()
	var path := _unique_path(POSTCARD_DIR, "postal-", ".png")
	if img.save_png(path) == OK:
		ui.toast("Postal guardada en %s" % path)
	else:
		ui.toast("No se pudo guardar la postal.")
	if not was_appreciation:
		_set_appreciation(false)


# --------------------------------------------------------------------------
# render con IA (fase 2)
# --------------------------------------------------------------------------

func _request_ai_render() -> void:
	if AIRender.is_busy():
		AIRender.cancel()
		ui.set_ai_busy(false)
		ui.toast("Render cancelado.")
		return
	if AIRender.request_render(garden):
		ui.set_ai_busy(true, "enviando")


func _on_ai_finished(stream: AudioStream, info: Dictionary) -> void:
	ui.set_ai_busy(false)
	if _rendered_player == null:
		_rendered_player = AudioStreamPlayer.new()
		add_child(_rendered_player)
		# Al acabar la pieza se devuelve el control al secuenciador en vivo.
		_rendered_player.finished.connect(func():
			Conductor.play()
			ui.set_playing(true))
	# El secuenciador se pausa: ahora se está escuchando la pieza terminada, no
	# construyéndola. Solaparlos sonaría a las dos cosas a la vez.
	Conductor.pause()
	ui.set_playing(false)
	_rendered_player.stream = stream
	_rendered_player.play()
	_set_appreciation(true)

	var origen := String(info.get("proveedor_real", "?"))
	if bool(info.get("fallback", false)):
		origen += " (fallback)"
	elif bool(info.get("desde_cache", false)):
		origen += " (caché)"
	ui.toast("Reproduciendo: %s · %.0f s" % [origen, float(info.get("duracion_s", 0.0))])


## Payload de la fase 2: prompt estructurado derivado de la escena real.
func _save_payload() -> void:
	if garden.count() == 0:
		ui.toast("El jardín está vacío.")
		return
	var json_path := _unique_path(PAYLOAD_DIR, "payload-", ".json")
	if not _write_json(json_path, garden.ai_payload()):
		ui.toast("No se pudo guardar el payload.")
		return
	# El .mid acompaña al JSON con el mismo nombre: uno es el prompt
	# estructurado, el otro el condicionamiento melódico.
	var midi_path := json_path.replace("payload-", "melodia-").replace(".json", ".mid")
	var f := FileAccess.open(midi_path, FileAccess.WRITE)
	if f == null:
		ui.toast("JSON guardado; falló el .mid.")
		return
	f.store_buffer(garden.to_midi())
	f.close()
	ui.toast("Payload + .mid en %s" % PAYLOAD_DIR)
