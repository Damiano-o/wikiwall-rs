//! Embed the tray/icon asset and the version strings in the Windows executable.

fn main() {
    println!("cargo:rerun-if-changed=assets/wikiwall.ico");
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/wikiwall.ico");
        resource.set("ProductName", "WikiWall");
        resource.set(
            "FileDescription",
            "Una voce eccellente da Wikipedia, sul tuo desktop" // niente accenti: rc.exe legge il .rc come ANSI,
        );
        resource.set(
            "LegalCopyright",
            "MIT - testi e immagini: Wikipedia CC BY-SA 4.0",
        );
        resource.set("OriginalFilename", "wikiwall.exe");
        if let Err(err) = resource.compile() {
            // The icon is cosmetic: never fail a build over it.
            println!("cargo:warning=icona e risorse non incluse: {err}");
        }
    }
}
