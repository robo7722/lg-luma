---
tags: [f1-hub, webos, tv-app, anleitung, prompt]
status: bereit
datum: 2026-11-18
bezug: "[[f1-hub-tv-app-konzept]]"
---

# Anleitung: F1 Hub TV-App (IPK) in einem neuen Chat bauen

## Schritt 1: Repo anlegen (du, auf github.com)

1. <https://github.com/new> öffnen.
2. Owner `robo7722`, Name **`F1-TV`**, **Public**.
3. Häkchen bei **„Add a README file“** setzen. Damit gibt es einen `main`-Branch, den die Session klonen kann.
4. **Create repository**.

## Schritt 2: Neue Claude-Code-Session starten

- Auf claude.ai/code eine neue Session starten und als Repository **`robo7722/F1-TV`** auswählen.
- Optional: `robo7722/F1` als zweites Repo hinzufügen (nur lesen reicht). Ist es öffentlich, kann die Session es aber auch ohne das klonen.

## Schritt 3: Diesen Prompt einfügen

```text
Baue in diesem Repo (robo7722/F1-TV) eine LG-webOS-TV-App (IPK) aus meinem F1 Hub. Arbeite selbstständig bis zum fertigen Ergebnis, committe und pushe auf deinen Arbeitsbranch und öffne am Ende einen Pull Request nach main.

## Quelle
- Original: https://github.com/robo7722/F1 (öffentlich, klonen mit `git clone --depth 1 https://github.com/robo7722/F1`).
- Es ist eine einzige index.html (Vanilla-JS, ES5-Stil, ca. 58 KB) mit den Tabs Termine, WM, Ergebnisse, News sowie Hero + Countdown zum nächsten GP. Daten: api.jolpi.ca (Ergast), api.openf1.org, api.open-meteo.com, en.wikipedia.org (F2-Tabellen), flagcdn.com, Streckenumrisse von bacinger/f1-circuits.
- Kopiere index.html und die Icons hierher. NICHT übernehmen: news.py, .github/workflows/news.yml, sw.js, manifest.json.

## News: weiterhin aus dem Original
- Die TV-App erzeugt KEINE eigenen News. loadNews() lädt stattdessen
  https://raw.githubusercontent.com/robo7722/F1/main/news.json?t=<Date.now()>
  (raw.githubusercontent.com sendet Access-Control-Allow-Origin: *). Das Format bleibt {at, items:[{s,title,src,u,t}]}.
- Bei einem Fehler den bestehenden localStorage-Cache anzeigen.

## TV-Design (1920×1080, Abstand ca. 3 m)
- Fest auf 1920×1080 (Viewport width=1920), Querformat, schwarzer Hintergrund, F1-Rot #e10600 als Akzent, Stil wie im Original (kursive Großbuchstaben, schräge Tabs).
- Safe Area: rundherum ca. 48 px horizontal und 27 px vertikal frei lassen.
- Schrift mindestens 24 px für Fließtext, Überschriften 36–56 px, Countdown groß (ca. 72 px).
- Layout: Oben eine Leiste mit Logo, Tabs (Termine · WM · Ergebnisse · News) und rechts Uhrzeit.
  Darunter zweispaltig: links (ca. 560 px) fest der Hero mit nächstem Termin, Flaggen-Hintergrund, Countdown und Wetter. Rechts der Inhalt des Tabs, scrollbar.
  - Termine: GP-Karten im Raster (2 Spalten), Filter Alle/F1/F2, „Vergangene zeigen“, Saison 2027.
  - WM: Unter-Tabs F1 Fahrer/F1 Teams/F2 Fahrer/F2 Teams, Tabelle in 2 Spalten (Plätze 1–12 | 13–24), damit sie ohne Scrollen passt.
  - Ergebnisse: Rennen als Karten. OK klappt die Karte auf (details/summary), darin Sprint/Quali/Rennen.
  - News: große Schlagzeilen-Karten im Raster (2–3 Spalten) mit Serie, Zeit und Quelle. Keine Links öffnen (OK macht nichts oder zeigt die Karte hervorgehoben).
- Entfernen bzw. ausblenden: Bearbeiten-Modus, Modal „Termin“, „+ Termin“, „Zurücksetzen“ (auf dem TV gibt es keine Tastatur). Die Termin-Daten (DEF, Live-Abgleich mit Jolpica) bleiben.
- Keine Hover-Effekte als einzige Rückmeldung. Kein backdrop-filter (zu teuer auf TV-GPUs), wenige und kurze Animationen.

## Bedienung mit der Fernbedienung
- Eigene Spatial Navigation (keine Bibliothek): Alle bedienbaren Elemente bekommen tabindex="0" bzw. sind Buttons. Pfeiltasten (37/38/39/40) suchen das nächste fokussierbare Element in der Richtung (Abstand der Rechteck-Mittelpunkte, Richtungsachse gewichtet).
- Fokus deutlich zeigen: 4 px weißer Rahmen + leichtes Vergrößern (transform: scale(1.03)), dazu roter Schatten. Nach jedem Fokuswechsel das Element mit scrollIntoView({block:"nearest"}) bzw. weich in den sichtbaren Bereich bringen.
- OK = Enter (13) löst click aus. Links/Rechts in der Tab-Leiste wechselt den Tab sofort.
- Zurück-Taste: keyCode 461 (webOS) und Escape/Backspace (Desktop-Test). Reihenfolge: offene Karte schließen → Fokus zurück auf die Tab-Leiste → App verlassen mit window.close() (Fallback: webOS.platformBack(), falls webOSTV.js eingebunden ist, sonst einfach nichts tun).
- Beim Start liegt der Fokus auf dem aktiven Tab. Der zuletzt gewählte Tab wird in localStorage gemerkt.
- Maus/Magic-Remote-Zeiger muss weiterhin funktionieren (Klicks).

## Kompatibilität
- Ziel: webOS 6 und neuer (Chromium 79+). webOS 4/5 (Chromium 53/68) soll zumindest starten.
- Kein ES2017+ ohne Not: kein optional chaining, kein ?? , keine Klassenfelder, kein CSS color-mix() ohne vorher gesetzten Fallback-Wert in einer eigenen Zeile, kein ::details-content, kein interpolate-size.
- Kein Service Worker (die App läuft von file://). Die Registrierung entfernen.
- Alle Daten wie bisher in localStorage cachen, damit beim Start sofort etwas zu sehen ist.

## webOS-Paket
- appinfo.json:
  {"id":"com.robo7722.f1hub","version":"1.0.0","vendor":"robo7722","type":"web","main":"index.html","title":"F1 Hub","icon":"icon.png","largeIcon":"largeIcon.png","bgColor":"#000000","resolution":"1920x1080","disableBackHistoryAPI":true}
- Icons aus icon-512.png erzeugen: icon.png 80×80, largeIcon.png 130×130 (z. B. mit sharp oder ImageMagick), Splash optional.
- App-Dateien in einen Ordner `app/`, Build-Ausgabe in `dist/` (in .gitignore).
- Build lokal: `npx -y @webos-tools/cli ares-package app -o dist` (falls der Paketname/Befehl anders heißt, recherchieren und anpassen). Ergebnis: dist/com.robo7722.f1hub_1.0.0_all.ipk.
- package.json mit Skripten: `build` (ares-package), `check` (ares-package --check bzw. JSON-Validierung von appinfo.json).

## GitHub Action
- .github/workflows/build.yml: bei Push und Pull Request die IPK bauen und als Artifact hochladen. Bei Tags v* zusätzlich ein Release mit der IPK anlegen (softprops/action-gh-release).
- Die Version in appinfo.json muss zum Tag passen. Prüfe das im Workflow.

## Test (in der Session, nicht auf dem TV)
- Mit dem vorinstallierten Playwright/Chromium index.html bei 1920×1080 öffnen und Screenshots von jedem Tab machen (deviceScaleFactor 1). Die Fernbedienung per Pfeiltasten/Enter/Escape simulieren und prüfen, dass jedes Element erreichbar ist und der Fokus immer sichtbar bleibt.
- Netzwerkzugriffe dürfen im Test echt sein. Wenn ein Host gesperrt ist, mit gemockten Antworten testen und das im Ergebnis sagen.
- Screenshots unter docs/ ablegen und im README einbinden. Kein Verhalten auf echter Hardware behaupten.

## README (Deutsch)
- Was die App ist und woher die News kommen (Original-Repo robo7722/F1).
- Installation:
  a) Developer Mode: `ares-setup-device`, dann `ares-install dist/*.ipk`.
  b) Gerooteter TV (Homebrew Channel): IPK per URL aus dem Release installieren oder per SSH:
     `luna-send -n 1 -f luna://com.webos.appInstallService/dev/install '{"id":"com.robo7722.f1hub","ipkUrl":"/tmp/f1hub.ipk","subscribe":true}'`
- Die App erscheint danach in der App-Liste von Luma bzw. im LG-Launcher.
- Bedienung mit der Fernbedienung (Tasten-Übersicht).

## Am Ende
- Liste, was getestet wurde und was nur auf dem echten TV geprüft werden kann.
- Pull Request nach main mit Screenshots öffnen.
```

## Hinweise

- Mein Fernseher bzw. meine webOS-Version gegebenenfalls im Prompt ergänzen (z. B. „LG C1, webOS 6“). Dann kann die Session die Kompatibilität gezielt darauf abstimmen.
- Die News-Action im Original-Repo `robo7722/F1` muss weiterlaufen. Sie bleibt die einzige Quelle für `news.json`.
