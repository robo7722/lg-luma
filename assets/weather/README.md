# Weather icons

Original Luma artwork under the project's MIT license: flat, coloured condition icons
(clear day/night, partly cloudy day/night, cloudy, fog, drizzle, rain, snow, storm).

The SVG files are the sources. `node tools/weather-icons.mjs` renders them to the 256 px PNGs
the launcher loads and records their SHA-256 hashes in `checksums.json`. No icons are fetched at
runtime; weather data comes from Open-Meteo.
