//! Embeds the icon and version information in the Windows executable.

fn main() {
    println!("cargo:rerun-if-changed=../../packaging/icon.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../packaging/icon.ico");
        res.set("ProductName", "HappyGreaseFloppy");
        res.set("FileDescription", "HappyGreaseFloppy - Greaseweazle front-end");
        if let Err(e) = res.compile() {
            println!("cargo:warning=Windows icon not embedded: {e}");
        }
    }
}
