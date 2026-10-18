# Vela Music desktop (Linux)

This portable package contains the native Linux client. It connects to an existing
Vela server origin; it does not start the server. You can explicitly save individual
owned tracks as private, account-scoped offline copies (Linux playback works with the
server stopped; there is no offline queue, background sync or removal UI).

Run `bin/vela-desktop`. Sign in with an account created by your Vela administrator.
The client uses the device's native audio output, keeps credentials only in process
memory and releases temporary prepared audio on replacement/sign-out/exit.

The package is built for x86_64 Linux and requires GTK3, WebKitGTK 4.1 and ALSA at
runtime. It is not a signed distro package or app-store build. See
`share/doc/vela-desktop/README.md` in the repository for verification scope and
known limitations.
