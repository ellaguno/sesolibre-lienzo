class_name MainUI
extends CanvasLayer

## Toda la UI se construye por código. Para un MVP evita mantener .tscn a mano y
## mantiene el layout en un solo sitio legible.

signal tool_selected(type_id: String)
signal transport_toggled()
signal seed_requested()
signal clear_requested()
signal mood_changed(mood_key: String)
signal bpm_changed(bpm: float)
signal appreciation_toggled()
signal gallery_toggled()
signal postcard_requested()
signal save_requested()
signal load_requested(path: String)
signal payload_requested()
signal ai_render_requested()
signal biome_changed(biome_id: String)
signal transpose_changed(semitones: int)

const PAD := 12

var _tool_buttons: Dictionary = {}
var _palette_row: HBoxContainer
var _play_btn: Button
var _mood_btn: OptionButton
var _ai_btn: Button
var _biome_btn: OptionButton
var _transpose: HSlider
var _transpose_lbl: Label
var _stats: Label
var _toast: Label
var _root: Control
var _top: Control
var _bottom: Control
var _gallery: PanelContainer
var _gallery_list: VBoxContainer
var _toast_tween: Tween


func _ready() -> void:
	layer = 10
	_root = Control.new()
	_root.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_root)

	_build_top()
	_build_palette()
	_build_stats()
	_build_gallery()
	_build_toast()


# --------------------------------------------------------------------------

func _btn(text: String, cb: Callable, min_w: float = 0.0) -> Button:
	var b := Button.new()
	b.text = text
	b.focus_mode = Control.FOCUS_NONE
	b.custom_minimum_size.x = min_w
	b.pressed.connect(cb)
	return b


func _build_top() -> void:
	# Las dos filas viven en un mismo contenedor para que el modo apreciación
	# pueda desvanecerlas como una sola cosa.
	var stack := VBoxContainer.new()
	stack.set_anchors_and_offsets_preset(Control.PRESET_TOP_WIDE)
	stack.offset_left = PAD
	stack.offset_right = -PAD
	stack.offset_top = PAD
	stack.add_theme_constant_override("separation", 6)
	_root.add_child(stack)
	_top = stack

	var bar := HBoxContainer.new()
	bar.add_theme_constant_override("separation", 6)
	stack.add_child(bar)

	_play_btn = _btn("▶", func(): transport_toggled.emit(), 44)
	bar.add_child(_play_btn)
	bar.add_child(_btn("Semilla", func(): seed_requested.emit()))
	bar.add_child(_btn("Vaciar", func(): clear_requested.emit()))

	var biomes := OptionButton.new()
	biomes.focus_mode = Control.FOCUS_NONE
	for i in Biomes.BIOME_ORDER.size():
		biomes.add_item(String(Biomes.biome(Biomes.BIOME_ORDER[i])["nm"]), i)
	biomes.item_selected.connect(
		func(idx: int): biome_changed.emit(String(Biomes.BIOME_ORDER[idx])))
	bar.add_child(biomes)
	_biome_btn = biomes

	var moods := OptionButton.new()
	moods.focus_mode = Control.FOCUS_NONE
	for i in Mapping.MOOD_ORDER.size():
		moods.add_item(String(Mapping.MOODS[Mapping.MOOD_ORDER[i]]["nm"]), i)
	moods.selected = Mapping.MOOD_ORDER.find("atardecer")
	moods.item_selected.connect(
		func(idx: int): mood_changed.emit(String(Mapping.MOOD_ORDER[idx])))
	bar.add_child(moods)
	_mood_btn = moods

	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	bar.add_child(spacer)

	var bpm := HSlider.new()
	bpm.min_value = 44
	bpm.max_value = 100
	bpm.step = 1
	bpm.value = 68
	bpm.custom_minimum_size = Vector2(110, 0)
	bpm.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	bpm.focus_mode = Control.FOCUS_NONE
	bpm.value_changed.connect(func(v: float): bpm_changed.emit(v))
	bar.add_child(bpm)

	# Segunda fila: acciones que no se usan en cada gesto.
	var bar2 := HBoxContainer.new()
	bar2.add_theme_constant_override("separation", 6)
	stack.add_child(bar2)

	var bar3 := HBoxContainer.new()
	bar3.add_theme_constant_override("separation", 6)
	stack.add_child(bar3)
	var tl := Label.new()
	tl.text = "Tono"
	tl.add_theme_font_size_override("font_size", 11)
	tl.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	bar3.add_child(tl)
	_transpose = HSlider.new()
	_transpose.min_value = Mapping.TRANSPOSE_MIN
	_transpose.max_value = Mapping.TRANSPOSE_MAX
	_transpose.step = 1
	_transpose.value = 0
	_transpose.custom_minimum_size = Vector2(150, 0)
	_transpose.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	_transpose.focus_mode = Control.FOCUS_NONE
	_transpose.value_changed.connect(func(v: float):
		set_transpose_label(int(v))
		transpose_changed.emit(int(v)))
	bar3.add_child(_transpose)
	_transpose_lbl = Label.new()
	_transpose_lbl.add_theme_font_size_override("font_size", 11)
	_transpose_lbl.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	bar3.add_child(_transpose_lbl)
	set_transpose_label(0)

	bar2.add_child(_btn("Apreciación", func(): appreciation_toggled.emit()))
	bar2.add_child(_btn("Galería", func(): gallery_toggled.emit()))
	bar2.add_child(_btn("Postal", func(): postcard_requested.emit()))
	bar2.add_child(_btn("Payload IA", func(): payload_requested.emit()))
	_ai_btn = _btn("Render IA", func(): ai_render_requested.emit())
	bar2.add_child(_ai_btn)


func set_transpose_label(v: int) -> void:
	_transpose_lbl.text = "0" if v == 0 else "%+d st" % v


func set_biome(biome_id: String) -> void:
	var idx := Biomes.BIOME_ORDER.find(biome_id)
	if idx >= 0:
		_biome_btn.selected = idx
	build_palette(biome_id)


func _build_palette() -> void:
	var scroll := ScrollContainer.new()
	scroll.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_WIDE)
	scroll.offset_top = -104
	scroll.offset_bottom = -PAD
	scroll.offset_left = PAD
	scroll.offset_right = -PAD
	scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_root.add_child(scroll)
	_bottom = scroll

	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 6)
	row.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.alignment = BoxContainer.ALIGNMENT_CENTER
	scroll.add_child(row)
	_palette_row = row


## La paleta depende del bioma, así que se reconstruye al cambiarlo.
func build_palette(biome_id: String) -> void:
	var row := _palette_row
	for c in row.get_children():
		c.queue_free()
	_tool_buttons.clear()
	for type_id in Biomes.objects_of(biome_id):
		var t: Dictionary = Mapping.TYPES[type_id]
		var b := Button.new()
		b.focus_mode = Control.FOCUS_NONE
		b.toggle_mode = true
		b.custom_minimum_size = Vector2(68, 72)
		b.text = "%s\n%s" % [String(t["nm"]), String(t["sub"])]
		b.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		b.add_theme_font_size_override("font_size", 10)
		b.add_theme_color_override("font_color", Mapping.color_of(type_id))
		b.add_theme_color_override("font_pressed_color", Color.WHITE)
		b.pressed.connect(func(): tool_selected.emit(type_id))
		row.add_child(b)
		_tool_buttons[type_id] = b


func _build_stats() -> void:
	_stats = Label.new()
	# TOP_WIDE en vez de TOP_RIGHT: con TOP_RIGHT el ancho es 0 y el texto
	# alineado a la derecha se sale del rectángulo.
	_stats.set_anchors_and_offsets_preset(Control.PRESET_TOP_WIDE)
	_stats.offset_left = PAD
	_stats.offset_right = -PAD
	_stats.offset_top = PAD + 96
	_stats.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	_stats.add_theme_font_size_override("font_size", 11)
	_stats.add_theme_color_override("font_color", Color(1, 1, 1, 0.62))
	_root.add_child(_stats)


func _build_toast() -> void:
	_toast = Label.new()
	_toast.set_anchors_and_offsets_preset(Control.PRESET_CENTER_BOTTOM)
	_toast.offset_bottom = -120
	_toast.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_toast.modulate.a = 0.0
	_toast.add_theme_font_size_override("font_size", 13)
	_root.add_child(_toast)


func _build_gallery() -> void:
	_gallery = PanelContainer.new()
	_gallery.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
	_gallery.custom_minimum_size = Vector2(300, 320)
	_gallery.visible = false
	_root.add_child(_gallery)

	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 8)
	_gallery.add_child(box)

	var title := Label.new()
	title.text = "Galería"
	title.add_theme_font_size_override("font_size", 16)
	box.add_child(title)

	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	box.add_child(scroll)
	_gallery_list = VBoxContainer.new()
	_gallery_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(_gallery_list)

	var actions := HBoxContainer.new()
	box.add_child(actions)
	actions.add_child(_btn("Guardar este jardín", func(): save_requested.emit()))
	actions.add_child(_btn("Cerrar", func(): gallery_toggled.emit()))


# --------------------------------------------------------------------------
# API
# --------------------------------------------------------------------------

func set_tool(type_id: String) -> void:
	for id in _tool_buttons:
		(_tool_buttons[id] as Button).button_pressed = (id == type_id)


## Sincroniza el desplegable cuando el momento cambia por otra vía (cargar un
## jardín de la galería). Asignar `selected` no reemite item_selected, así que no
## hay riesgo de bucle.
func set_mood(mood_key: String) -> void:
	var idx := Mapping.MOOD_ORDER.find(mood_key)
	if idx >= 0:
		_mood_btn.selected = idx


func set_ai_busy(busy: bool, label := "") -> void:
	_ai_btn.disabled = busy
	_ai_btn.text = label if busy else "Render IA"


func set_playing(playing: bool) -> void:
	_play_btn.text = "❚❚" if playing else "▶"


func set_stats(garden: Garden) -> void:
	# La duración del ciclo depende de los pasos del bioma, no de una constante.
	var cycle := (Mapping.cycle_beats(garden.biome_id, garden.steps())
			* 60.0 / maxf(garden.bpm, 1.0))
	var layers: Array[String] = []
	for k in garden.active_layers():
		if bool(garden.active_layers()[k]):
			layers.append(String(k))
	_stats.text = "%s · %d objetos · %.0f%%\n%s · %d BPM\nciclo %.1f s\n%s" % [
		String(Biomes.biome(garden.biome_id)["nm"]),
		garden.count(), garden.density() * 100.0,
		Mapping.scale_name(garden.biome_id, garden.mood_key, garden.transpose), int(round(garden.bpm)), cycle,
		"capas: " + ", ".join(layers) if not layers.is_empty() else "sin capas",
	]


func set_appreciation(on: bool) -> void:
	# La UI no se destruye, se desvanece: volver debe ser inmediato.
	var tw := create_tween().set_parallel(true)
	for node in [_top, _bottom, _stats]:
		tw.tween_property(node, "modulate:a", 0.0 if on else 1.0, 0.45)
	_top.mouse_filter = Control.MOUSE_FILTER_IGNORE if on else Control.MOUSE_FILTER_PASS
	_bottom.mouse_filter = Control.MOUSE_FILTER_IGNORE if on else Control.MOUSE_FILTER_STOP


func toggle_gallery(entries: Array[Dictionary]) -> bool:
	_gallery.visible = not _gallery.visible
	if _gallery.visible:
		for child in _gallery_list.get_children():
			child.queue_free()
		if entries.is_empty():
			var empty := Label.new()
			empty.text = "Todavía no hay jardines guardados."
			empty.add_theme_font_size_override("font_size", 12)
			_gallery_list.add_child(empty)
		for e in entries:
			var path := String(e["path"])
			var b := _btn("%s · %d objetos" % [String(e["titulo"]), int(e["objetos"])],
				func(): load_requested.emit(path))
			b.alignment = HORIZONTAL_ALIGNMENT_LEFT
			_gallery_list.add_child(b)
	return _gallery.visible


func toast(text: String) -> void:
	_toast.text = text
	if _toast_tween != null and _toast_tween.is_valid():
		_toast_tween.kill()
	_toast.modulate.a = 1.0
	_toast_tween = create_tween()
	_toast_tween.tween_interval(1.8)
	_toast_tween.tween_property(_toast, "modulate:a", 0.0, 0.6)


## true si el puntero está sobre un control: sirve para no plantar detrás de la UI.
func blocks_pointer(pos: Vector2) -> bool:
	if _gallery.visible and _gallery.get_global_rect().has_point(pos):
		return true
	if _top.modulate.a > 0.5:
		for node in [_top, _bottom]:
			if (node as Control).get_global_rect().has_point(pos):
				return true
	return false
