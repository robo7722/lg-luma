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
