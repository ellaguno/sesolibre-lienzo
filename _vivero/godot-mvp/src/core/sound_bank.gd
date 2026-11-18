extends Node

## Banco de muestras, buses y capas globales. Autoload `SoundBank`.
##
## Las muestras son placeholders sintetizados (ver tools/gen_samples.py). Sirven
## para validar la estructura; el pack real lo graba un compositor.

const AUDIO_DIR := "res://assets/audio/"
## Todos los loops declarados por los biomas, más los del bosque por defecto.
const BASE_LOOPS := ["drone_loop", "air_loop"]

var _streams: Dictionary = {}
var _lowpass: AudioEffectLowPassFilter
var _reverb: AudioEffectReverb

var _drone: AudioStreamPlayer
var _air: AudioStreamPlayer
var _pulse: AudioStreamPlayer
var _drone_started := false

var _mood_key := "atardecer"
var _biome_id := "bosque"

# El filtro y la reverb tienen DOS fuentes: la densidad de la escena y la
# posición de la cámara. Se guardan por separado y se combinan, porque si cada
# una escribiera directo la última en llegar pisaría a la otra.
var _density_cut := 1100.0
var _persp_cut_factor := 1.0
var _mood_wet := 0.45
var _persp_wet := 0.0
var _persp_db := 0.0

var _tweens: Dictionary = {}


func _ready() -> void:
	_load_streams()
	_setup_master_bus()
	_setup_layers()
	set_environment(_biome_id, _mood_key)
	set_density(0, 80)


func _exit_tree() -> void:
	# Las capas son loops infinitos: si siguen sonando al cerrar, sus playbacks
	# retienen los streams y el motor los reporta como fugas.
	for p in [_drone, _air, _pulse]:
		if p != null and p.playing:
			p.stop()
	_streams.clear()
	_tweens.clear()


func _load_streams() -> void:
	# La lista se deriva del catálogo: añadir un objeto con una muestra nueva no
	# obliga a tocar este archivo.
	var wanted := {}
	for type_id in Biomes.OBJECTS:
		wanted[String(Biomes.OBJECTS[type_id]["sample"])] = true
	for l in BASE_LOOPS:
		wanted[l] = true
	for bid in Biomes.MUSIC:
		wanted[String(Biomes.MUSIC[bid]["drone"])] = true
		wanted[String(Biomes.MUSIC[bid]["air"])] = true

	for entry in wanted:
		var sample_name := String(entry)
		var path: String = AUDIO_DIR + sample_name + ".wav"
		var stream := load(path)
		if stream == null:
			push_error("SoundBank: falta %s. Corre: python3 tools/gen_samples.py" % path)
			continue
		if sample_name.ends_with("_loop") and stream is AudioStreamWAV:
			var wav := stream as AudioStreamWAV
			wav.loop_mode = AudioStreamWAV.LOOP_FORWARD
			wav.loop_begin = 0
			wav.loop_end = int(wav.get_length() * float(wav.mix_rate))
		_streams[sample_name] = stream


func _setup_master_bus() -> void:
	var master := AudioServer.get_bus_index("Master")
	# Orden importante: primero se filtra la fuente, después se reverbera. Al
	# revés, el filtro se comería la cola de la reverb.
	_lowpass = AudioEffectLowPassFilter.new()
	_lowpass.cutoff_hz = 1100.0
	AudioServer.add_bus_effect(master, _lowpass)

	_reverb = AudioEffectReverb.new()
	_reverb.room_size = 0.88
	_reverb.damping = 0.35
	_reverb.spread = 0.85
	_reverb.predelay_msec = 20.0
	_reverb.dry = 1.0
	_reverb.wet = 0.45
	AudioServer.add_bus_effect(master, _reverb)


func _setup_layers() -> void:
	_drone = _make_layer("drone_loop")
	_air = _make_layer("air_loop")
	_pulse = AudioStreamPlayer.new()
	_pulse.stream = _streams.get("sub")
	_pulse.volume_db = -60.0
	add_child(_pulse)


func _make_layer(sample: String) -> AudioStreamPlayer:
	var p := AudioStreamPlayer.new()
	p.stream = _streams.get(sample)
	p.volume_db = -60.0
	add_child(p)
	# NO se arranca aquí: un loop sonando a -60 dB mientras se ajusta su
	# pitch_scale produce un glissando audible al abrir la app.
	return p


func stream_for(type_id: String) -> AudioStream:
	return _streams.get(String(Biomes.OBJECTS[type_id]["sample"]))


func set_environment(biome_id: String, mood_key: String) -> void:
	var biome_changed := biome_id != _biome_id
	_biome_id = biome_id
	_mood_key = mood_key
	_mood_wet = Biomes.reverb_of(biome_id, mood_key)
	_apply_wet()

	# El fondo continuo es lo que más tiempo está en el oído: si los siete biomas
	# comparten drone y aire, suenan igual por mucho que cambien los timbres.
	if biome_changed:
		var mus: Dictionary = Biomes.music(biome_id)
		_swap_layer(_drone, String(mus["drone"]))
		_swap_layer(_air, String(mus["air"]))

	if _drone:
		# El pitch se fija SIEMPRE antes de arrancar el loop, nunca con rampa.
		var target := Mapping.root_of(biome_id, mood_key, 0) - 24
		_drone.pitch_scale = pow(2.0, float(target - 33) / 12.0)


func _swap_layer(player: AudioStreamPlayer, sample_name: String) -> void:
	if player == null:
		return
	var stream: AudioStream = _streams.get(sample_name)
	if stream == null or player.stream == stream:
		return
	var was_playing := player.playing
	player.stream = stream
	if was_playing:
		player.play()


## Capas y filtro en función del número de objetos. Es la única variable de
## "calidad" de una escena: más densidad = más tapiz, menos = más aire.
func set_density(count: int, total_cells: int) -> void:
	var th := Mapping.layer_thresholds(total_cells)
	var d := clampf(float(count) / maxf(1.0, float(total_cells) * 0.45), 0.0, 1.0)
	_density_cut = 900.0 + d * 8200.0
	_apply_filter()

	if count >= int(th["drone"]) and not _drone_started:
		_drone_started = true
		_drone.play()
		_air.play()
	_fade(_drone, "volume_db", -18.0 if count >= int(th["drone"]) else -60.0, 3.0)
	_fade(_air, "volume_db", -21.0 if count >= int(th["aire"]) else -60.0, 4.0)
	_pulse.volume_db = -13.0 if count >= int(th["pulso"]) else -60.0


## La cámara ES la posición de escucha.
##
## `top_down` 0..1 (0 = a ras de la escena, 1 = cenital) y `distance` 0..1.
## Mirar desde arriba y desde lejos es una vista de conjunto: se oye más
## apagado, más reverberado y algo más bajo. Acercarse a ras deja el sonido
## seco y presente. NO cambia la composición — solo la perspectiva de escucha —,
## y por eso puede estar siempre activo sin estorbar a quien solo quiere mirar.
func set_perspective(top_down: float, distance: float) -> void:
	var far := maxf(top_down, distance)
	_persp_cut_factor = lerpf(1.0, 0.42, far)
	_persp_wet = lerpf(0.0, 0.20, far)
	_persp_db = lerpf(0.0, -3.5, far)
	_apply_filter()
	_apply_wet()
	AudioServer.set_bus_volume_db(AudioServer.get_bus_index("Master"), _persp_db)


func _apply_filter() -> void:
	if _lowpass:
		_fade(_lowpass, "cutoff_hz",
			clampf(_density_cut * _persp_cut_factor, 300.0, 20000.0), 0.5)


func _apply_wet() -> void:
	if _reverb:
		_fade(_reverb, "wet", clampf(_mood_wet + _persp_wet, 0.05, 0.95), 0.8)


## El pulso lento lo dispara el Conductor una vez por ciclo (ver main.gd).
func pulse(mood_key: String) -> void:
	if _pulse.volume_db <= -59.0:
		return
	var target := int(Biomes.MOODS[mood_key]["root"]) - 24
	_pulse.pitch_scale = pow(2.0, float(target - 36) / 12.0)
	_pulse.play()


func _fade(target: Object, property: String, value: float, secs: float) -> void:
	if target == null:
		return
	var key := "%d:%s" % [target.get_instance_id(), property]
	var prev: Tween = _tweens.get(key)
	if prev != null and prev.is_valid():
		prev.kill()
	var tw := create_tween()
	tw.tween_property(target, property, value, secs)
	_tweens[key] = tw
