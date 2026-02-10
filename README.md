# Whisperdict

<p align="center">
  Local voice dictation for your desktop. Press a shortcut, speak, and paste text where you are working.
</p>

<p align="center">
  Inspired by Superwhisper, built for Linux, and designed for full privacy with 100% local processing.
</p>

<p align="center">
  <a href="https://github.com/nabby27/whisperdict/actions/workflows/release.yml"><img alt="Release workflow" src="https://img.shields.io/github/actions/workflow/status/nabby27/whisperdict/release.yml?label=release"></a>
  <a href="https://github.com/nabby27/whisperdict/releases"><img alt="Latest release" src="https://img.shields.io/github/v/release/nabby27/whisperdict?sort=semver"></a>
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2.x-24C8DB?logo=tauri&logoColor=white">
  <img alt="React" src="https://img.shields.io/badge/React-18-149ECA?logo=react&logoColor=white">
  <img alt="TypeScript" src="https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript&logoColor=white">
  <a href="./LICENSE"><img alt="MIT license" src="https://img.shields.io/badge/License-MIT-green.svg"></a>
</p>

<video src="./public/assets/demo.mp4" controls muted playsinline width="100%"></video>

## What is Whisperdict?

Whisperdict is a desktop app built with Tauri, Rust, and React that gives you quick voice dictation with local Whisper models. It is inspired by Superwhisper, focused on Linux users, and designed to stay out of your way:

- Press your global shortcut.
- Speak naturally.
- Get transcription pasted into your active app.

No browser tab juggling, no context switching.

## Superwhisper-style experience for Linux, with full privacy

- Inspired by the fast workflow popularized by Superwhisper.
- Tailored for Linux desktop usage.
- All transcription runs locally on your machine.
- Your audio does not need to leave your device.

## Why people use it

- Fast dictation flow with a global shortcut.
- Local model options (`tiny` to `large`) with in-app download and management.
- Language selection for multilingual dictation.
- Clipboard + paste automation so text lands where you are working.
- Lightweight desktop UI to monitor status, progress, and last transcript.

## How it works

1. Whisperdict listens for your configured global shortcut (default `Ctrl+Alt+Space`).
2. It records from your microphone.
3. Audio is transcribed using the selected local Whisper model.
4. Transcribed text is emitted to the app and can be pasted into your focused target.

## Installation

### Download binaries

Grab the latest release from:

- https://github.com/nabby27/whisperdict/releases

## App behavior and defaults

- Default shortcut: `Ctrl+Alt+Space`
- Default model: `base`
- Default language: `en`
- Free usage counter starts at `50` transcriptions

## Tech stack

- Tauri 2 (desktop shell)
- Rust backend (`whisper-rs`, audio capture, hotkeys, paste automation)
- React + TypeScript frontend (Vite)
- GitHub Actions release pipeline

## Contributing

Issues and PRs are welcome. If you find a bug or want a feature, open an issue with:

- OS and desktop session details
- Steps to reproduce
- Expected vs actual behavior

## License

This project is licensed under the MIT License. See `LICENSE`.
