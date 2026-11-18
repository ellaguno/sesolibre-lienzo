extends SceneTree

## Exporta el catálogo a ../shared/catalog.json.
##
##     godot --headless --path godot-mvp --script tools/export_catalog.gd
##
## `Biomes.gd` es la única fuente de verdad. El servicio de render (Python) NO
## puede importar GDScript, así que antes duplicaba las tablas a mano — y al
## añadir cuatro biomas la copia quedó obsoleta al instante y el servicio empezó
## a rechazar sus payloads. Este archivo generado es el contrato entre ambos.
##
## Hay que volver a correrlo después de tocar `Biomes.gd`. La prueba de
## `ai-render` avisa si el catálogo se ha quedado atrás.

const OUT_REL := "../shared/catalog.json"


func _init() -> void:
	var catalog := {
		"version": 2,
		"generado_por": "godot-mvp/tools/export_catalog.gd",
		"paso_en_pulsos": Mapping.STEP_BEATS,
		"transposicion": {"min": Mapping.TRANSPOSE_MIN, "max": Mapping.TRANSPOSE_MAX},
		# Fracciones de la rejilla, no cuentas fijas: 16 objetos no significan lo
		# mismo en 60 celdas que en 100.
		"capas": {
			"drone_frac": Mapping.LAYER_DRONE_FRAC,
			"aire_frac": Mapping.LAYER_AIR_FRAC,
			"pulso_frac": Mapping.LAYER_PULSE_FRAC,
			"drone_min": Mapping.LAYER_DRONE_MIN,
			"aire_min": Mapping.LAYER_AIR_MIN,
			"pulso_min": Mapping.LAYER_PULSE_MIN,
		},
		"variacion": {
			"periodos_largo": Mapping.PERIODS_LONG,
			"periodos_medio": Mapping.PERIODS_MID,
			"periodos_corto": Mapping.PERIODS_SHORT,
			"arco_segundos": Mapping.ARC_SECONDS,
			"deriva_segundos": Mapping.DRIFT_SECONDS,
			"repeticion_objetivo_s": Mapping.TARGET_REPEAT_SEC,
			"prob_caracter": Mapping.FEEL_PROB,
		},
		"musica": Biomes.MUSIC,
		"objetos": Biomes.OBJECTS,
		"momentos": Biomes.MOODS,
		"orden_momentos": Biomes.MOOD_ORDER,
		"biomas": Biomes.BIOMES,
		"orden_biomas": Biomes.BIOME_ORDER,
	}

	var base := ProjectSettings.globalize_path("res://")
	var path := base.path_join(OUT_REL).simplify_path()
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())

	var f := FileAccess.open(path, FileAccess.WRITE)
	if f == null:
		push_error("No se pudo escribir %s (error %d)" % [path, FileAccess.get_open_error()])
		quit(1)
		return
	f.store_string(JSON.stringify(catalog, "  "))
	f.close()

	print("catálogo -> %s" % path)
	print("  %d objetos, %d momentos, %d biomas" % [
		Biomes.OBJECTS.size(), Biomes.MOODS.size(), Biomes.BIOMES.size()])
	quit()
