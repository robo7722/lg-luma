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

| Baustein | Umsetzung |
| --- | --- |
| Quellcode | Branch `alt` in diesem Repo (oder eigenes Repo `lg-luma-alt`), regelmäßig von `main` gemergt |
| Releases | eigene Tags `alt-vX.Y.Z` und eigener Feed `repo-alt.json` (oder eigenes Repo) |
| Kanalwahl | in Luma: **Einstellungen → System → Update-Kanal: Stabil / Alternativ** |
| Wechsel | Kanal umschalten → Luma lädt die neueste IPK des anderen Kanals und installiert sie über das vorhandene `self-update.sh` |
| Zurück | denselben Schalter zurückstellen. Notfalls die Stabil-IPK per SSH oder Dev Manager installieren |

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
