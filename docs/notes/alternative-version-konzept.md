---
tags: [luma, webos, konzept, update-kanal]
status: entwurf
datum: 2026-11-18
---

# Alternative Luma-Version als „Update“ mit einfachem Zurückwechseln

## Was der Code heute festlegt

- App-ID `com.ecklf.luma`, Programmdatei `luma` (`appinfo.json`, wird in `tools/package.mjs` erzwungen).
- Takeover (`home.sh`), Autostart (`autostart.sh`) und Self-Update (`tv/self-update.sh`)
  rechnen fest mit dem Installationspfad `.../applications/com.ecklf.luma`.
- Self-Update (`src/update.rs`) liest nur
  `https://github.com/robo7722/lg-luma/releases/latest/download/repo.json`,
  akzeptiert nur IPKs von `robo7722/lg-luma` und bietet nur **strikt höhere** Versionen an.
- Einstellungen liegen **außerhalb** des App-Ordners (`/media/developer/...` bzw.
  `/media/internal/...`, `src/storage.rs`). Sie bleiben bei einer Neuinstallation also erhalten.

## Empfehlung: zweiter Update-Kanal mit derselben App-ID

Die alternative Version behält `com.ecklf.luma`. Der Fernseher sieht sie dann als ganz normales Update:
Takeover, Autostart, Input-Hook-Befehl und Einstellungen funktionieren weiter, ohne dass etwas umgestellt werden muss.

| Baustein  | Umsetzung                                                                                                                |
| --------- | ------------------------------------------------------------------------------------------------------------------------ |
| Quellcode | Branch `alt` in diesem Repo (oder eigenes Repo `lg-luma-alt`), regelmäßig von `main` gemergt                             |
| Releases  | eigene Tags `alt-vX.Y.Z` und eigener Feed `repo-alt.json` (oder eigenes Repo)                                            |
| Kanalwahl | in Luma: **Einstellungen → System → Update-Kanal: Stabil / Alternativ**                                                  |
| Wechsel   | Kanal umschalten → Luma lädt die neueste IPK des anderen Kanals und installiert sie über das vorhandene `self-update.sh` |
| Zurück    | denselben Schalter zurückstellen. Notfalls die Stabil-IPK per SSH oder Dev Manager installieren                          |

### Nötige Codeänderungen

1. `update.rs`: Feed-URL und erlaubtes Release-Präfix hängen vom Kanal ab.
   Bei einem **ausdrücklichen Kanalwechsel** darf auch eine gleiche oder ältere Version installiert werden.
   Der normale Update-Check bleibt bei „nur neuer“.
2. Kanal-Einstellung in `Preferences` speichern. Die stabile Version muss das Feld kennen
   (es also beibehalten), damit der Wechsel in beide Richtungen funktioniert. Deshalb kommt diese Änderung zuerst in **beide** Kanäle.
3. `tools/repository.mjs` / CI: zweiter Build-Job für `alt`, der Tag und Feed mit `alt` kennzeichnet.
4. Optional: „ALT“-Badge bzw. Versionsanzeige in den Einstellungen, damit man sieht, welcher Kanal läuft.

### Risiken / offen

- **Downgrade über `appInstallService/dev/install`**: dass webOS eine ältere oder gleiche Version
  über eine neuere installiert, ist am Fernseher noch nicht geprüft. Vor dem Bau des Kanalschalters
  einmal manuell testen.
- **Einstellungsformat**: Wenn `alt` neue Felder einführt, verwirft die stabile Version sie beim Speichern.
  Daher: neue Felder nur additiv und optional, oder `alt`-spezifische Einstellungen in einer eigenen Datei.
- Homebrew Channel: Wer beide Feeds einträgt, bekommt dort konkurrierende Versionen derselben ID angezeigt.
  Daher nur einen Feed eintragen und den Wechsel über Luma selbst machen.

## Alternative: eigene App-ID (`com.ecklf.luma.alt`)

Damit laufen beide Versionen nebeneinander und es gibt null Risiko für die stabile Version. Das ist aber **kein** Update:
Takeover/Autostart müssten auf die neue ID umgestellt werden, und die Einstellungen werden nicht übernommen.
Das ist nur sinnvoll als Spielwiese, nicht für „nahtlos“.

## Update 2026-11-18: Ziel ist ein 90er-VHS-Menü → Entscheidung Modus statt App

Gewünscht ist ein alternativer **Anzeigemodus** im Look eines 90er-Jahre-VHS-Menüs.
Empfehlung: **umschaltbarer Modus in Luma** (Einstellungen → General → Design: Liquid Glass / VHS),
keine separate App und kein zweiter Update-Kanal.

Gründe:

- Es ändert sich nur das Aussehen. App-Liste, Eingänge, Starten, Takeover und Einstellungen bleiben gleich.
  Eine zweite App würde all das duplizieren, und beide Versionen würden auseinanderlaufen.
- Umschalten dauert eine Sekunde, ohne Installation und ohne Downgrade-Risiko.
- Es gibt schon ein Vorbild: `WeatherStyle` (`src/model.rs`) ist eine gespeicherte Stilwahl mit Vorschau-Picker.
  Ein `UiTheme`-Enum funktioniert genauso.
- Gezeichnet wird zentral in `src/view.rs`/`src/renderer.rs`. Der VHS-Modus bekommt eigene Zeichenfunktionen
  für Home/Drawer/Settings. Navigation und Zustand (`src/state.rs`) bleiben unverändert.

Mögliche VHS-Elemente:

- blauer Hintergrund, Blockschrift in Weiß/Gelb und ein OSD-Look wie „PLAY ▶“ oder „CH 03“
- Menüpunkte als Liste mit blinkendem Cursor statt Glas-Karten
- Scanlines, leichtes Farbflimmern und Tracking-Rauschen beim Seitenwechsel als Shader (`src/shaders/`)
- Uhr im Videorekorder-Stil mit blinkendem „12:00“-Gag, dazu passende Pieps-Sounds

Separate App nur dann, wenn der VHS-Modus später eine komplett andere Bedienlogik bekommen soll.

## Update: VHS-Home bekommt ein eigenes Layout

Anforderung: Im VHS-Modus soll mindestens der **Hauptbildschirm anders aufgebaut** sein, nicht nur anders eingefärbt.
Es bleibt trotzdem beim Modus statt einer separaten App. Der VHS-Home ist eine eigene Seiten-Variante.

Entwurf (Videorekorder-OSD, 1920×1080):

```
 ┌──────────────────────────────────────────────────────────┐
 │  ▶ PLAY                                       12:00  SP  │  OSD-Kopfzeile (Uhr, Wetter als „18°C KLAR“)
 │                                                          │
 │            ═══  M A I N   M E N U  ═══                   │
 │                                                          │
 │        ▶ 01  NETFLIX                                      │  Favoriten als nummerierte Liste
 │          02  YOUTUBE                                      │  ▶-Cursor blinkt, Auswahl gelb
 │          03  PLEX                                         │
 │          04  ALLE PROGRAMME …                             │  → App-Drawer (ebenfalls als Liste)
 │          05  EINGANG  (HDMI 1 / HDMI 2 …)                 │
 │          06  EINSTELLUNGEN                                │
 │                                                          │
 │  CH 03                        ◀◀ REW   ▶▶ FF   ■ STOP     │  OSD-Fußzeile mit Tastenhinweisen
 └──────────────────────────────────────────────────────────┘
   + Scanlines, leichtes Bildrauschen, „Tracking“-Wackler bei Seitenwechsel
```

Technik:

- `view.rs`: `home()` verzweigt nach Design. Für VHS kommt `vhs_home()` dazu.
- `state.rs`: Auf dem VHS-Home ist die Navigation **Hoch/Runter durch die Liste** statt Links/Rechts durch den Dock.
  Die vorhandenen `Target`s (Nav, InputsMenu, SettingsMenu …) werden wiederverwendet.
  Halten von OK und Verschieben funktionieren in der Liste vertikal.
- Ziffertasten 1–9 der Fernbedienung springen direkt zum Eintrag (passt zum Rekorder-Feeling).
- Drawer, Settings und Recents bleiben in Phase 1 im normalen Look, bekommen aber den VHS-Farbfilter oder Scanlines.
  Ein vollständiger VHS-Umbau folgt später.
- Schrift: Pixel- bzw. OSD-artige Monospace-Schrift mit Lizenz unter `assets/fonts/`.
  Muss die Sprachen abdecken oder für CJK auf die bisherige Schrift zurückfallen.

## Skill-Recherche VHS/Retro-UI (2026-11-18)

Gesucht wurde nach „vhs, retro ui, crt, scanlines, 90s, pixel font, glitch“ in den claude.ai-Skills und im Plugin-Verzeichnis.

- **Es gibt keinen Skill und kein Plugin speziell für VHS- oder Retro-Look.**
- Brauchbar ist **`webgl-shaders`** (bereits aktiv): GLSL-Post-Processing, Rauschen, Effekte.
  Er ist für WebGL im Browser geschrieben, aber das GLSL-Wissen passt auf Lumas OpenGL ES 2 (`src/shaders/*.frag`).
  Nützlich für Scanlines, Farbsaum (Chroma-Versatz), Tracking-Rauschen und Bildflackern.
- Allgemeine Design-Plugins (Anthropic „Design“, Superdesign, Refero) haben keinen Retro-Bezug und bringen für ein natives TV-UI wenig.
- Falls gewünscht, schreiben wir einen eigenen Projekt-Skill `vhs-look`
  (Farbpalette, OSD-Typografie, Shader-Rezepte, Do/Don't), damit spätere Sitzungen den Stil konsistent halten.

## Internet-Recherche VHS/Retro (2026-11-18)

### Skills

Online gibt es **keinen Skill speziell für VHS-UI**. Gefunden wurden nur Verwandte, alle aus Drittanbieter-Verzeichnissen:

- `bergside/awesome-design-skills` → **retro** (MIT): wenig Substanz. Generische Web-Palette (Blau/Lila), Schrift Macondo/JetBrains Mono, keine Effektregeln. Für uns kaum brauchbar.
- **tui-designer** (ckorhonen): Terminal-/CRT-Look mit Scanlines, Glow und Wölbung, CSS-Ansatz, Bernstein-CRT-Palette. Die Effekt-Ideen sind gut, aber für Web/Terminal gedacht.
- **windows-3-1-web-designer**, **vintage**, **vaporwave**, **retro-futurism**: falsche Epoche oder falscher Stil.
- **visual-console-eras** (Blender): nennt VHS, ist aber Art Direction für 3D-Szenen und „Unverified“.
  → Fazit: Ein eigener `vhs-look`-Skill im Repo ist sinnvoller als eine Installation von außen.

### Shader-Vorlagen (für `src/shaders/`, GLSL ES 1.0)

- Godot Shaders „VHS and CRT monitor effect“, **CC0**: Scanlines, Grille, Verfärbung, Rauschen, Rolling Distortion.
- Godot Shaders „Realistic CRT shader“, **CC0**: Scanlines (nach Timothy Lottes), Rauschen, horizontale Störungen, Chroma-Versatz.
  Läuft laut Nutzerbericht unter GLES2.
- „VHS Scanline Glitch“, „Lofi Retro Camera Filter“: Lizenz bzw. GLES2 unklar, nur als Ideenquelle nutzen.
  Godot-Shader müssen auf reines GLSL portiert werden (`SCREEN_TEXTURE`/`TIME` → eigene Uniforms).
  Performance auf alten webOS-GPUs muss am TV geprüft werden.

### Schrift

- **VCR OSD Mono**: das Original, Lizenz aber **ungeklärt** („free“) → nicht ins Repo aufnehmen.
- **Better VCR** (artdzyk 2023): **SIL OFL 1.1**, laut Anbieter 68 Sprachen inkl. Kyrillisch/Polnisch.
  Der beste Kandidat. Upstream-Quelle und genaue Glyphen-Abdeckung müssen vor der Aufnahme geprüft werden.
  Für CJK/Vietnamesisch auf die bisherige Schrift zurückfallen.
- VCR OSD Neue: nur „bitte nennen“, keine klare Lizenz → nicht verwenden.

## Skill `vhs-look` angelegt (2026-11-18)

- `.agents/skills/vhs-look/SKILL.md`: Regeln (Idle = nichts animieren, gleiche Navigation, ein Design-Schalter),
  Palette (`vhs.blue #1418A0`, `vhs.yellow #E8D02E` …, broadcast-safe), OSD-Typografie (Großbuchstaben, Monospace-Raster,
  harter Schatten), Home-Layout, Bewegung (keine Glides, 1-Hz-Cursor, 180–240 ms Bandwechsel-Glitch), Pieptöne,
  OLED-Schutz (±2 px Pixel-Shift), Code-Landkarte, Prüfschritte, Verbote.
- `.agents/skills/vhs-look/references/effects.md`: GLES2-Rezepte für Scanlines, Tracking-Glitch, Rauschtextur, Chroma-Saum,
  Vignette, abgestimmt auf Lumas Teil-Neuzeichnung (`src/damage.rs`).
- `.claude/skills` → Symlink auf `.agents/skills`, damit Claude Code die Projekt-Skills automatisch findet.

## Stilfrage: VCR-OSD vs. „90er Kabel-Wetterkanal“ (2026-11-18)

Der Nutzer hat als Referenz einen Screenshot gezeigt („O'Brien's Weather Station“, YouTube-Stream).
Das ist **nicht** das VCR-OSD-Menü, sondern der Look der **90er-Jahre-Kabel-TV-Infokanäle**:

- bekannt als „Local on the 8s“ / WeatherStar-4000-Look (The Weather Channel, ca. 1990–1998)
- Oberbegriffe: _Retro Broadcast Graphics_, _90s Cable TV Aesthetic_, _Weather Channel Aesthetic_
- Merkmale: blaue Verlaufs-Panels mit hellem Rahmen/Fase, Kopfzeile mit Titel-Box und Uhr-Box,
  fette abgerundete Grotesk in Gelb mit dickem schwarzem Rand/Schatten, weiße Labels links + gelbe Werte rechts,
  Pixel-Art-Icons, Laufschrift-Ticker unten, feine Scanlines.
- Fan-Nachbau als Open Source: „WeatherStar 4000+“ (Web-Emulation). Nur als Stilreferenz,
  keine Marken, Logos oder 1:1-Layouts übernehmen.

Zwischenstand Code (WIP, Branch `ccr-39c15a48-9biymt`): `Design`-Präferenz (Glass/Vhs), Übersetzungen,
vertikale Home-Navigation + Zifferntasten + Blink-Takt sind drin. Die VHS-Ansicht selbst ist noch nicht gebaut
und die Einstellung noch nicht sichtbar. Je nach Stilentscheidung wird `vhs_home()` als OSD-Liste oder als Broadcast-Panel-Layout gezeichnet.

## Entscheidung: „Retro TV“ statt VHS, Phase 1 umgesetzt (2026-11-18)

- Der Nutzer will einen **stilisierten** Retro-Look mit der Stimmung der 90er-Kabel-TV-Infokanäle, keine Kopie.
- Der Modus heißt jetzt **Retro TV** (`Preferences::design = "retro"`), umschaltbar unter Einstellungen → Allgemein → Design.
- **Wetter nur unten im Ticker.** Die Box links zeigt das gewählte App-Icon als kleinen Röhrenmonitor („CH 01“).
- Startseite: Titel-Panel „HAUPTMENÜ“, Uhr-Panel mit Datum, nummerierte Liste (Favoriten, Alle Apps, Eingänge,
  Luma-Einstellungen, TV-Einstellungen), 9 Zeilen pro Seite, Zifferntasten 1–9, ▶-Cursor blinkt, Verschieben mit Hoch/Runter,
  Ticker mit Hinweisen, Wetter und Datum (Wechsel alle 8 s), Scanlines, kurzes Bildrauschen beim Erscheinen.
- App-Übersicht, Einstellungen und „Zuletzt verwendet“ bleiben vorerst im Liquid-Glass-Look.
- Skill umbenannt: `.agents/skills/retro-look/`.
- Vorschau-Screenshots (Desktop, keine TV-Aussage): ![[retro-home.png]] ![[retro-allapps.png]]

## Feedback 2: Kopfzeile nur „HAUPTMENÜ“ (2026-11-18)

- Wetterkanal-/Infokanal-Look und Umbenennung „Retro TV“ sind vom Nutzer bestätigt.
- Test: Oben nur „HAUPTMENÜ“ ohne Uhr-/Datums-Panel → vom Nutzer **verworfen** („davor war es besser“).
  Es bleibt beim Titel-Panel „HAUPTMENÜ“ plus Uhr-Panel mit Datum rechts.
- **Arbeitsregel:** Vor jedem Commit/Push zuerst ein Design-Bild zeigen und das OK abwarten.

## Retro-Töne und Musik über ElevenLabs (2026-11-18, Entwurf, noch nicht freigegeben)

Ablage: `tools/sound-design/retro/` (wird nicht ins IPK gepackt).

- `elevenlabs/`: Rohdateien, Sound Effects v2 (`eleven_text_to_sound_v2`, prompt_influence 0.6), je 2 Kandidaten:
  focus (Synth-Blip), select (2 steigende Beeps), back (2 fallende Beeps), bump (dumpfes Brummen), tracking (VHS-Rauschen, 0,8 s).
- `variants/`: zugeschnitten (Stille vorne weg, focus 0,10 s, select/back 0,22 s, bump 0,18 s, tracking 0,45 s), 48 kHz Mono 16 bit,
  Lautheit (mean volume) an die bisherigen Töne angeglichen: focus −26 dB, select −22,4, back −20,7, bump −18,4, tracking −24, Limiter 0,89.
- `demo-sounds-1/2.wav`: nachgestellte Bedienung (3× Fokus, Anschlag, Zurück, Fokus, Auswahl, Rauschen) bei halbem Pegel wie in der App.
- `music/infochannel-1.mp3` (Smooth Jazz, E-Piano, Fretless-Bass, Sax, 90 BPM) und `infochannel-2.mp3` (Synth-Pop/New Age, 100 BPM),
  je 60 s, `music_v1`.
- Kosten: Musik ca. 56 Credits pro 60 s, Effekte kaum messbar. Stand: ca. 5.900 Credits übrig (Starter-Tarif, kommerzielle Nutzung laut Tarif erlaubt,
  vor Veröffentlichung ElevenLabs-Bedingungen prüfen).

Offene Fragen an den Nutzer: welches Ton-Set, welche Musik? Soll die Musik nur auf der Retro-Startseite laufen
(aus beim App-Start, eigener Schalter, standardmäßig aus)?

## Feedback 3: Uhr und Datum unten links (2026-11-18, Entwurf)

- Oben nur „HAUPTMENÜ“ (Titel-Panel volle Breite). Uhrzeit und Datum stehen jetzt im orangen Feld unten links
  (vorher „LUMA TV“, Feld auf 460 px verbreitert). Das Datum ist aus der Ticker-Rotation entfernt.

## Entscheidung Töne/Musik (2026-11-18)

- Nutzerwahl: **Ton-Set 1**, **Musik 1** (Smooth Jazz). Der Musikschalter kommt wie vorgeschlagen:
  nur auf der Retro-Startseite, eigener Schalter unter Allgemein (nur im Retro-Design sichtbar), standardmäßig aus,
  **stoppt beim App-Start** (sowie beim Verlassen der Startseite und im Hintergrund) und spielt danach an derselben Stelle weiter.
- Umsetzung: `assets/sounds/retro-{focus,select,back,bump,tracking}.wav`, `retro-music.wav` (24 kHz Mono, ca. 57 s,
  ca. 2,7 MB, nahtlose Schleife), Mischung im SDL-Audio-Callback (`src/audio.rs`), Einstellung `retro_music`.

## Launcher-Einstellungen im Retro-Look (2026-11-18, Entwurf)

- Wunsch: Im Retro-Modus soll auch das Launcher-Menü (Luma-Einstellungen) den Look bekommen.
- Umsetzung als Restyle-Filter (`retro_view::restyle`) über die fertige Einstellungs-Szene: Layout, Ziele und Navigation
  bleiben identisch (Test vergleicht die Controls). Wallpaper → Navy, Glas → blaues Verlaufs-Panel mit hellem Rahmen,
  fokussiertes Glas → Gelb, Schatten/Glasränder weg, Ecken max. 6 px, Schrift fett (800) mit hartem Schatten und
  automatisch auf die vorhandene Breite verkleinert, Scanlines. Gilt für alle Bereiche und Unterseiten.

## Vorfall: 3.7.0 friert ein, keine Steuerung (2026-11-18)

- Nach dem Update auf 3.7.0 reagierte Luma nicht mehr auf die Fernbedienung. Das ist dasselbe wie damals bei 3.6.2.
- Ursache: 3.7.0 baute auf `main` mit der 3.6.2-Änderung auf (Ton-Ausgabe bei Vordergrund und Audiogeräte-Events neu öffnen).
  Die damals vorbereitete Rücknahme 3.6.3 wurde nie veröffentlicht.
- Fix 3.7.1 (lokal vorbereitet): Ton-Ausgabe wieder nur einmal beim Start öffnen (wie 3.6.1), kein `resume`/`reconnect` aus der
  Event-Schleife, Musik setzt nur ein Flag. Retro-Funktionen bleiben.
- **Regel:** Änderungen an Ton-Ausgabe/Event-Schleife erst per PR-Test-IPK am TV prüfen, dann veröffentlichen.

## 3.7.1 veröffentlicht, nächste Wünsche (2026-11-18)

- Test-IPK 3.7.1 lief am TV („funktioniert wieder“). PR #14 wurde gemergt, 3.7.1 wird veröffentlicht.
- Neue Wünsche, lokal umgesetzt, noch nicht hochgeladen:
  1. **Hintergrundmusik automatisch an** beim Umschalten auf Retro TV (bleibt abschaltbar).
  2. **„Alle Apps“ als Retro-Fenster mit Auflistung**: Titel „ALLE APPS“, nummerierte Liste der Apps aus der App-Übersicht
     (ohne Favoriten), Monitor-Vorschau links, Seiten à 9, Zifferntasten, Verschieben mit Hoch/Runter, Ticker unten.
     Die Musik läuft dort weiter.
  3. **Animierte Bildfehler** (Wunsch: Scanlines leicht animiert, alte TV-Bildfehler): langsam durchlaufender Brummbalken
     (9 s pro Durchlauf, sehr dezent) und etwa alle 12 s eine 0,3 s kurze Tracking-Störung mit Flackern.
     Retro-Seiten zeichnen dafür mit ~12 fps nur den betroffenen Streifen neu. Bei „Bewegungen reduzieren“ ist alles aus.
     Vorschau-Video: `scratchpad/retro-faults.mp4` (Desktop).

## 3.8.0 Rückmeldung: Animation nur im Sekundentakt (2026-11-18)

- Am TV bewegte sich der Brummbalken nur etwa einmal pro Sekunde oder wenn die Magic Remote bewegt wurde.
- Ursache (vermutet): `SDL_WaitEventTimeout(100 ms)` schläft unter SDL-webOS deutlich länger.
- Fix 3.8.1: Während der Bildfehler schläft die Hauptschleife selbst in ≤10-ms-Scheiben bis zum nächsten Schritt und pollt SDL nur noch.
  Wegen der Regel für Event-Schleifen-Änderungen: erst Test-IPK am TV, dann veröffentlichen.
