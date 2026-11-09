//! En Windows, pega el ícono de NicoStation al reproductor (lo que se ve en el Explorador y la barra de tareas).
fn main() {
    println!("cargo:rerun-if-changed=assets/icono.ico");
    println!("cargo:rerun-if-changed=assets/icono.rc");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("icono.o");
    let windres = if cfg!(windows) { "windres" } else { "x86_64-w64-mingw32-windres" };
    let ok = std::process::Command::new(windres)
        .args(["--input", "assets/icono.rc", "--output-format=coff", "--include-dir", "assets", "--output"])
        .arg(&out)
        .status()
        .is_ok_and(|s| s.success());
    if ok {
        println!("cargo:rustc-link-arg-bin=nicostation={}", out.display());
    } else {
        println!("cargo:warning=no pude compilar el ícono con {windres}; el .exe queda sin ícono");
    }
}
