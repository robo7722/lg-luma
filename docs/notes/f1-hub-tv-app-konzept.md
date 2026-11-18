---
tags: [f1-hub, webos, tv-app, konzept]
status: entwurf
datum: 2026-11-18
quelle: robo7722/F1 (Stand fc0e28b)
---

# F1 Hub als App für den LG-Fernseher

## Ist-Stand des F1 Hub (`robo7722/F1`)

- Eine einzige Datei `index.html` (~58 KB, Vanilla-JS, kein Build-Schritt), dazu PWA-Teile
  (`manifest.json`, `sw.js`, Icons).
- Tabs: **Termine**, **WM**, **Ergebnisse**, **News**, dazu ein Hero-Bereich mit Countdown zum nächsten GP.
- Daten kommen direkt aus dem Browser von: Jolpica/Ergast (`api.jolpi.ca`), OpenF1, Open-Meteo,
  Wikipedia, `flagcdn.com` und den Streckenumrissen `bacinger/f1-circuits`.
- News: GitHub Action (`news.yml`, alle 20 Min.) führt `news.py` aus (Google News + motorsport.com RSS)
  und committet `news.json`.
- Ausgelegt fürs Handy: `orientation: portrait`, `.wrap{max-width:640px}`, Bedienung per Touch/Klick.

## Was für den Fernseher fehlt

1. **Layout für 1920×1080 aus 3 m Abstand**: Querformat, größere Schrift, mehrspaltig
   (z. B. links Hero/Countdown, rechts Liste), Safe-Area-Ränder von etwa 5 %.
2. **Bedienung mit der Fernbedienung**: Fokus-Navigation mit den Pfeiltasten (Spatial Navigation),
   deutlich sichtbarer Fokusrahmen, OK = Enter, **Zurück = keyCode 461** (schließt Modal/Detail,
   sonst die App beenden). Tabs per Links/Rechts erreichbar.
3. **Keine Hover- oder Touch-Abhängigkeit**: Bearbeiten-Modus und Modal müssen sich ohne Maus bedienen lassen.
4. **News-Links**: Auf dem TV ergibt das Öffnen externer Artikel wenig Sinn → Titel und Quelle anzeigen,
   OK öffnet höchstens eine einfache Lesansicht oder nichts.

## Empfehlung: webOS-Web-App (IPK) + TV-Modus im bestehenden Hub

Kein Nachbau in Rust/Luma. Der Hub bleibt eine Codebasis:

| Baustein | Umsetzung |
| --- | --- |
| TV-Modus | im `index.html` des F1-Repos: aktiv bei `?tv=1` oder `Web0S` im User-Agent, setzt `html.tv` und eigene CSS-Regeln plus ein kleines Fokus-Navigationsskript |
| Paket | eigene IPK, z. B. `com.robo7722.f1hub`, `appinfo.json` mit `type: "web"`, `resolution: 1920x1080`, Icons 80×80/130×130 |
| Inhalt der IPK | **Variante A (empfohlen):** nur eine Start-Seite, die auf die GitHub-Pages-URL mit `?tv=1` weiterleitet → jede Änderung am Hub und jede neue `news.json` ist sofort auf dem TV, ohne neue IPK. **Variante B:** Hub-Dateien in die IPK bündeln (startet ohne Netz schneller, Updates nur per neuer IPK) |
| Installation | wie Luma per SSH/`luna-send` oder Dev Manager auf den gerooteten TV; taucht dann in Lumas App-Drawer auf und kann in die Favoriten-Leiste |
| Build | GitHub Action im F1-Repo baut die IPK (`ares-package` oder schlichtes `ar`/`tar` wie `tools/package.mjs` in Luma) und hängt sie ans Release |

### Offene Punkte

- Ist GitHub Pages für `robo7722/F1` aktiv? (Voraussetzung für Variante A)
- Service Worker auf webOS: funktioniert ab webOS 4 in Web-Apps. Wenn Probleme auftreten, im TV-Modus nicht registrieren.
- Ältere webOS-Chromium-Versionen (z. B. webOS 4/5 = Chromium 53/68): der Code verwendet bereits `var` und ES5-Stil, das sieht gut aus. Optional Chaining (`?.`) o. Ä. vorher prüfen.
- Für Änderungen am F1-Repo braucht die Session Schreibzugriff (Push) auf `robo7722/F1`.

## Aufwand (Schätzung)

- TV-Layout + Fokus-Navigation: der Hauptteil, etwa 300–400 Zeilen CSS/JS im `index.html`.
- IPK-Wrapper + Action: klein (appinfo.json, Icons, Start-Seite, Workflow).
