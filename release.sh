#!/usr/bin/env bash
cross build --target x86_64-pc-windows-gnu --release
cross build --target x86_64-unknown-linux-gnu --release
rm target/kristforge_{windows,linux}.zip
zip -j target/kristforge_windows.zip target/x86_64-pc-windows-gnu/release/kristforge.exe
zip -j target/kristforge_linux.zip target/x86_64-unknown-linux-gnu/release/kristforge
