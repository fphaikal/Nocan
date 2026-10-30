use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/mascot/runtime/nocan.ico");
    println!("cargo:rerun-if-env-changed=WINDRES");
    println!("cargo:rerun-if-env-changed=RC");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let assets = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("assets");
    let icon = assets.join("mascot/runtime/nocan.ico");
    let manifest = assets.join("nocan.manifest");
    println!("cargo:rerun-if-changed={}", manifest.display());
    let rc_file = out.join("nocan.rc");
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let numeric_version = version.split('.').take(3).collect::<Vec<_>>().join(",") + ",0";
    fs::write(
        &rc_file,
        format!(
            concat!(
                "1 ICON \"{}\"\n1 24 \"{}\"\n",
                "1 VERSIONINFO\nFILEVERSION {}\nPRODUCTVERSION {}\n",
                "FILEOS 0x40004L\nFILETYPE 0x1L\nBEGIN\n",
                " BLOCK \"StringFileInfo\"\n BEGIN\n  BLOCK \"040904b0\"\n  BEGIN\n",
                "   VALUE \"CompanyName\", \"Nocan contributors\\0\"\n",
                "   VALUE \"FileDescription\", \"Nocan desktop notch\\0\"\n",
                "   VALUE \"FileVersion\", \"{}\\0\"\n",
                "   VALUE \"InternalName\", \"nocan\\0\"\n",
                "   VALUE \"OriginalFilename\", \"nocan.exe\\0\"\n",
                "   VALUE \"ProductName\", \"Nocan\\0\"\n",
                "   VALUE \"ProductVersion\", \"{}\\0\"\n",
                "  END\n END\n BLOCK \"VarFileInfo\"\n BEGIN\n",
                "  VALUE \"Translation\", 0x0409, 1200\n END\nEND\n"
            ),
            icon.display().to_string().replace('\\', "/"),
            manifest.display().to_string().replace('\\', "/"),
            numeric_version, numeric_version, version, version,
        ),
    )
    .expect("write Nocan icon and manifest resource definition");

    let is_gnu = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu");
    let resource = out.join(if is_gnu { "nocan-resource.o" } else { "nocan.res" });
    let mut compiler = if is_gnu {
        Command::new(env::var_os("WINDRES").unwrap_or_else(|| "windres".into()))
    } else {
        Command::new(env::var_os("RC").unwrap_or_else(|| "rc".into()))
    };
    if is_gnu {
        compiler.arg("-i").arg(&rc_file).arg("-O").arg("coff").arg("-o").arg(&resource);
    } else {
        compiler.arg("/nologo").arg(format!("/fo{}", resource.display())).arg(&rc_file);
    }
    let status = compiler.status().expect("Windows resource compiler required (windres for GNU, rc for MSVC)");
    assert!(status.success(), "could not compile Nocan application icon");
    println!("cargo:rustc-link-arg-bin=nocan={}", resource.display());
}
