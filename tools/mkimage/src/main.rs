use std::{fs::OpenOptions, os::unix::fs::FileExt, sync::Arc};
use rcore_fs::{dev::{Device, DevError, Result}, vfs::{FileSystem, FileType}};
struct Disk(std::fs::File);
impl Device for Disk {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        self.0.read_at(buf, offset as u64).map_err(|_| DevError)
    }
    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        self.0.write_at(buf, offset as u64).map_err(|_| DevError)
    }
    fn sync(&self) -> Result<()> { self.0.sync_all().map_err(|_| DevError) }
}
fn main() {
    let path = std::env::args().nth(1).expect("disk path required");
    let file = OpenOptions::new().create(true).truncate(true).read(true).write(true).open(path).unwrap();
    file.set_len(64 * 1024 * 1024).unwrap();
    let fs = rcore_fs_sfs::SimpleFileSystem::create(Arc::new(Disk(file)), 64 * 1024 * 1024).unwrap();
    let swap = fs.root_inode().create("SWAP_FILE", FileType::File, 0o600).unwrap();
    swap.resize(16 * 1024 * 1024).unwrap();
    fs.sync().unwrap();
}
