use std::path::Path;

/// Unix sistemlerde dosya izinlerinin 0600 olup olmadığını denetler.
///
/// Grup veya diğer kullanıcılar için okuma/yazma/çalıştırma izinleri açıksa stderr'e
/// uyarı basar (`PLAN.md` Faz 1).
#[cfg(unix)]
pub fn check_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    if let Ok(metadata) = std::fs::metadata(path) {
        let mode = metadata.permissions().mode();
        let perm_bits = mode & 0o777;
        if (perm_bits & 0o077) != 0 {
            eprintln!(
                "warning: config file '{}' permissions are too open ({perm_bits:04o}). Expected 0600. Other users on this system may read your credentials.",
                path.display()
            );
        }
    }
}

#[cfg(not(unix))]
pub fn check_permissions(_path: &Path) {}

/// Unix sistemlerde dosya izinlerini 0600 olarak ayarlar.
#[cfg(unix)]
pub fn ensure_0600_permissions(path: &Path) -> Result<(), std::io::Error> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o600);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
pub fn ensure_0600_permissions(_path: &Path) -> Result<(), std::io::Error> {
    Ok(())
}
