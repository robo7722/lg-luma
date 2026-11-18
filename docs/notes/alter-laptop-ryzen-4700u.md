---
tags: [hardware, laptop, luma, entwicklung]
status: bewertung
datum: 2026-11-18
---

# Alter Laptop: Ryzen 7 4700U / 32 GB – Bewertung

## Hardware

| Komponente      | Spezifikation                         |
| --------------- | ------------------------------------- |
| Prozessor       | AMD Ryzen 7 4700U (Zen 2, 15 W TDP)   |
| CPU-Kerne       | 8 Kerne / 8 Threads (kein SMT)        |
| Arbeitsspeicher | 32 GB RAM                             |
| Grafik          | AMD Radeon Vega 7 (integriert)        |
| SSD             | 256 GB                                |
| Architektur     | x86-64                                |

## Eignung als Luma-Entwicklungsrechner

**Gut geeignet.** x86-64-Linux ist laut `development.md` eine unterstützte Nix-Plattform.

- **CPU/RAM:** 8 echte Kerne und 32 GB reichen locker für Rust-Builds (Desktop-Preview und
  ARMv7-Cross-Build). Der 15-W-Chip drosselt bei langen Builds, aber inkrementelle Builds
  bleiben schnell.
- **Grafik:** Vega 7 läuft unter Linux mit Mesa (`radeonsi`) und kann GLES2 nativ.
  `pnpm dev` läuft damit direkt auf dem Bildschirm statt nur unter Xvfb.
- **SSD (Engpass):** 256 GB werden knapp. Nix-Store, webOS-SDK/NDK, `.build/`,
  `target/` (Debug + ARM-Release) und `node_modules` brauchen zusammen gut 30–60 GB.
  Regelmäßig `cargo clean` und `nix-collect-garbage -d` ausführen.
- **Größter Vorteil:** Er steht im selben Netz wie der Fernseher. `pnpm deploy --device tv`
  (ares) braucht direkten LAN-Zugriff auf den TV – das kann die Cloud-Sitzung nicht.

### Vorschlag für die Einrichtung

1. Linux installieren (z. B. Fedora oder Ubuntu LTS; NixOS, wenn alles über Nix laufen soll).
2. Nix mit Flakes installieren, Repo klonen, `nix develop`.
3. `pnpm install --frozen-lockfile && pnpm assets && pnpm dev` als Test.
4. `pnpm sdk`, `pnpm runtime`, ares-Geräteprofil für den TV anlegen → `pnpm deploy --device tv`.
5. Optional: Claude-Desktop-App mit Remote Control, damit Claude-Sitzungen direkt auf dem
   Laptop (und damit am TV) arbeiten können.

## Andere sinnvolle Einsätze

- **Heimserver:** sparsam im Leerlauf; Jellyfin/Plex mit VAAPI-Hardware-Transcoding
  (H.264/HEVC) über die Vega-GPU, dazu Home Assistant, Pi-hole o. Ä.
- **Lokale KI-Modelle:** 32 GB RAM reichen für quantisierte 7–14B-Modelle (llama.cpp/Ollama),
  aber nur CPU-schnell – grob 5–10 Token/s bei 7–8B. Zum Ausprobieren ok, nicht für Alltag.
- **Nicht geeignet:** aktuelle Spiele, größere KI-Modelle, viel Speicherplatz ohne externe Platte.
