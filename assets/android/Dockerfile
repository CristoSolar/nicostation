FROM notfl3/cargo-apk:latest
# zip: lo usa cargo-quad-apk (master) para meter classes.dex en el APK
RUN pacman -Sy --noconfirm zip unzip && rm -rf /var/cache/pacman/pkg/*
# Rust moderno (rapier2d 0.36, Cargo.lock v4, crates edition 2024) + targets Android
RUN rustup update stable && rustup default stable \
 && rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
# cargo-quad-apk de git master (usa cargo 0.87: entiende lock v4 / edition 2024; el 0.1.4 de la imagen usa cargo 0.62)
RUN cargo install --locked --git https://github.com/not-fl3/cargo-quad-apk cargo-quad-apk \
 && rm -rf /usr/local/cargo/registry /usr/local/cargo/git
WORKDIR /root/src
