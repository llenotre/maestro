/// TODO doc

use core::ffi::c_void;
use crate::errno::Errno;
use crate::errno;
use crate::filesystem::File;
use crate::filesystem::path::Path;
use crate::filesystem;
use crate::process::Process;
use crate::util;

/// TODO doc
pub const O_APPEND: u32 =    0b00000000000001;
/// TODO doc
pub const O_ASYNC: u32 =     0b00000000000010;
/// TODO doc
pub const O_CLOEXEC: u32 =   0b00000000000100;
/// TODO doc
pub const O_CREAT: u32 =     0b00000000001000;
/// TODO doc
pub const O_DIRECT: u32 =    0b00000000010000;
/// TODO doc
pub const O_DIRECTORY: u32 = 0b00000000100000;
/// TODO doc
pub const O_EXCL: u32 =      0b00000001000000;
/// TODO doc
pub const O_LARGEFILE: u32 = 0b00000010000000;
/// TODO doc
pub const O_NOATIME: u32 =   0b00000100000000;
/// TODO doc
pub const O_NOCTTY: u32 =    0b00001000000000;
/// TODO doc
pub const O_NOFOLLOW: u32 =  0b00010000000000;
/// TODO doc
pub const O_NONBLOCK: u32 =  0b00100000000000;
/// TODO doc
pub const O_SYNC: u32 =      0b01000000000000;
/// TODO doc
pub const O_TRUNC: u32 =     0b10000000000000;

/// Returns the absolute path to the file.
fn get_file_absolute_path(process: &Process, path_str: &str) -> Result<Path, Errno> {
	let path = Path::from_string(path_str)?;
	if !path.is_absolute() {
		let cwd = process.get_cwd();
		let mut absolute_path = cwd.concat(&path)?;
		absolute_path.reduce()?;
		Ok(absolute_path)
	} else {
		Ok(path)
	}
}

fn get_file(path: Path, flags: u32) -> Result::<&'static mut File, Errno> {
	if let Some(file) = filesystem::get_file_from_path(&path) {
		Ok(file)
	} else {
		if flags & O_CREAT != 0 {
			// TODO Create file, return errno on fail
			Err(-errno::ENOENT as _)
		} else {
			Err(-errno::ENOENT as _)
		}
	}
}

/// The implementation of the `open` syscall.
pub fn open(regs: &util::Regs) -> u32 {
	let pathname = regs.ebx as *const c_void;
	let flags = regs.ecx;
	let _mode = regs.edx as u16;

	let mut curr_proc = Process::get_current().unwrap();
	// TODO Check that path is in process's memory
	// TODO Check path length (ENAMETOOLONG)
	let path_str = unsafe { // Call to unsafe function
		util::ptr_to_str(pathname)
	};

	// TODO Resolve symbolic links up to limit (if too many, ELOOP)

	let file_path = get_file_absolute_path(&curr_proc, path_str);
	if file_path.is_err() {
		return -errno::ENOMEM as _;
	}
	let file_path = file_path.unwrap();

	let file_result = get_file(file_path, flags);
	if let Err(errno) = file_result {
		-errno as _
	} else {
		let file = file_result.unwrap();
		let fd = curr_proc.open_file(file);
		if let Err(errno) = fd {
			-errno as _
		} else {
			fd.unwrap().get_id()
		}
	}
}