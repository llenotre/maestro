//! The ext2 filesystem is a classical filesystem used in Unix systems.
//! It is nowdays obsolete and has been replaced by ext3 and ext4.
//!
//! The filesystem divides the storage device into several substructures:
//! - Block Group: stored in the Block Group Descriptor Table (BGDT)
//! - Block: stored inside of block groups
//! - INode: represents a file in the filesystem
//!
//! The access to an INode data is divided into several parts, each overflowing on the next when
//! full:
//! - Direct Block Pointers: each inode has 12 of them
//! - Singly Indirect Block Pointer: a pointer to a block dedicated to storing a list of more
//! blocks to store the inode's data. The number of blocks it can store depends on the size of a
//! block.
//! - Doubly Indirect Block Pointer: a pointer to a block storing pointers to Singly Indirect Block
//! Pointers, each storing pointers to more blocks.
//! - Triply Indirect Block Pointer: a pointer to a block storing pointers to Doubly Indirect Block
//! Pointers, each storing pointers to Singly Indirect Block Pointers, each storing pointers to
//! more blocks.
//!
//! Since the size of a block pointer is 4 bytes, the maximum size of a file is:
//! `(12 * n) + ((n/4) * n) + ((n/4)^^2 * n) + ((n/4)^^3 * n)`
//! Where `n` is the size of a block.

use core::mem::MaybeUninit;
use core::mem::size_of;
use core::mem::size_of_val;
use core::slice;
use crate::device::DeviceHandle;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileType;
use crate::file::INode;
use crate::file::filesystem::Filesystem;
use crate::file::filesystem::FilesystemType;
use crate::file::path::Path;
use crate::file;
use crate::memory::malloc;
use crate::time;
use crate::util::boxed::Box;
use crate::util::container::string::String;
use crate::util::math;
use crate::util;

/// The offset of the superblock from the beginning of the device.
const SUPERBLOCK_OFFSET: u64 = 1024;
/// The filesystem's signature.
const EXT2_SIGNATURE: u16 = 0xef53;

/// Default filesystem major version.
const DEFAULT_MAJOR: u32 = 1;
/// Default filesystem minor version.
const DEFAULT_MINOR: u16 = 1;
/// Default filesystem block size.
const DEFAULT_BLOCK_SIZE: u64 = 1024;
/// Default inode size.
const DEFAULT_INODE_SIZE: u16 = 128;
/// Default inode size.
const DEFAULT_INODES_PER_GROUP: u32 = 1024;
/// Default number of blocks per block group.
const DEFAULT_BLOCKS_PER_GROUP: u32 = 1024;
/// Default number of mounts in between each fsck.
const DEFAULT_MOUNT_COUNT_BEFORE_FSCK: u16 = 1000;
/// Default elapsed time in between each fsck in seconds.
const DEFAULT_FSCK_INTERVAL: u32 = 16070400;

/// The block offset of the Block Group Descriptor Table.
const BGDT_BLOCK_OFFSET: usize = 2; // TODO Compute using block size

/// State telling that the filesystem is clean.
const FS_STATE_CLEAN: u16 = 1;
/// State telling that the filesystem has errors.
const FS_STATE_ERROR: u16 = 2;

/// Error handle action telling to ignore it.
const ERR_ACTION_IGNORE: u16 = 1;
/// Error handle action telling to mount as read-only.
const ERR_ACTION_READ_ONLY: u16 = 2;
/// Error handle action telling to trigger a kernel panic.
const ERR_ACTION_KERNEL_PANIC: u16 = 3;

/// Optional feature: Preallocation of a specified number of blocks for each new directories.
const OPTIONAL_FEATURE_DIRECTORY_PREALLOCATION: u32 = 0x1;
/// Optional feature: AFS server
const OPTIONAL_FEATURE_AFS: u32 = 0x2;
/// Optional feature: Journal
const OPTIONAL_FEATURE_JOURNAL: u32 = 0x4;
/// Optional feature: Inodes have extended attributes
const OPTIONAL_FEATURE_INODE_EXTENDED: u32 = 0x8;
/// Optional feature: Filesystem can resize itself for larger partitions
const OPTIONAL_FEATURE_RESIZE: u32 = 0x10;
/// Optional feature: Directories use hash index
const OPTIONAL_FEATURE_HASH_INDEX: u32 = 0x20;

/// Required feature: Compression
const REQUIRED_FEATURE_COMPRESSION: u32 = 0x1;
/// Required feature: Directory entries have a type field
const REQUIRED_FEATURE_DIRECTORY_TYPE: u32 = 0x2;
/// Required feature: Filesystem needs to replay its journal
const REQUIRED_FEATURE_JOURNAL_REPLAY: u32 = 0x4;
/// Required feature: Filesystem uses a journal device
const REQUIRED_FEATURE_JOURNAL_DEVIXE: u32 = 0x8;

/// Write-required feature: Sparse superblocks and group descriptor tables
const WRITE_REQUIRED_SPARSE_SUPERBLOCKS: u32 = 0x1;
/// Write-required feature: Filesystem uses a 64-bit file size
const WRITE_REQUIRED_64_BITS: u32 = 0x2;
/// Directory contents are stored in the form of a Binary Tree.
const WRITE_REQUIRED_DIRECTORY_BINARY_TREE: u32 = 0x4;

/// The maximum number of direct blocks for each inodes.
const DIRECT_BLOCKS_COUNT: usize = 12;

/// INode type: FIFO
const INODE_TYPE_FIFO: u16 = 0x1000;
/// INode type: Char device
const INODE_TYPE_CHAR_DEVICE: u16 = 0x2000;
/// INode type: Directory
const INODE_TYPE_DIRECTORY: u16 = 0x4000;
/// INode type: Block device
const INODE_TYPE_BLOCK_DEVICE: u16 = 0x6000;
/// INode type: Regular file
const INODE_TYPE_REGULAR: u16 = 0x8000;
/// INode type: Symbolic link
const INODE_TYPE_SYMLINK: u16 = 0xa000;
/// INode type: Socket
const INODE_TYPE_SOCKET: u16 = 0xc000;

/// User: Read, Write and Execute.
const INODE_PERMISSION_IRWXU: u16 = 00700;
/// User: Read.
const INODE_PERMISSION_IRUSR: u16 = 00400;
/// User: Write.
const INODE_PERMISSION_IWUSR: u16 = 00200;
/// User: Execute.
const INODE_PERMISSION_IXUSR: u16 = 00100;
/// Group: Read, Write and Execute.
const INODE_PERMISSION_IRWXG: u16 = 00070;
/// Group: Read.
const INODE_PERMISSION_IRGRP: u16 = 00040;
/// Group: Write.
const INODE_PERMISSION_IWGRP: u16 = 00020;
/// Group: Execute.
const INODE_PERMISSION_IXGRP: u16 = 00010;
/// Other: Read, Write and Execute.
const INODE_PERMISSION_IRWXO: u16 = 00007;
/// Other: Read.
const INODE_PERMISSION_IROTH: u16 = 00004;
/// Other: Write.
const INODE_PERMISSION_IWOTH: u16 = 00002;
/// Other: Execute.
const INODE_PERMISSION_IXOTH: u16 = 00001;
/// Setuid.
const INODE_PERMISSION_ISUID: u16 = 04000;
/// Setgid.
const INODE_PERMISSION_ISGID: u16 = 02000;
/// Sticky bit.
const INODE_PERMISSION_ISVTX: u16 = 01000;

/// Secure deletion
const INODE_FLAG_SECURE_DELETION: u32 = 0x00001;
/// Keep a copy of data when deleted
const INODE_FLAG_DELETE_COPY: u32 = 0x00002;
/// File compression
const INODE_FLAG_COMPRESSION: u32 = 0x00004;
/// Synchronous updates
const INODE_FLAG_SYNC: u32 = 0x00008;
/// Immutable file
const INODE_FLAG_IMMUTABLE: u32 = 0x00010;
/// Append only
const INODE_FLAG_APPEND_ONLY: u32 = 0x00020;
/// File is not included in 'dump' command
const INODE_FLAG_NODUMP: u32 = 0x00040;
/// Last accessed time should not updated
const INODE_FLAG_ATIME_NOUPDATE: u32 = 0x00080;
/// Hash indexed directory
const INODE_FLAG_HASH_INDEXED: u32 = 0x10000;
/// AFS directory
const INODE_FLAG_AFS_DIRECTORY: u32 = 0x20000;
/// Journal file data
const INODE_FLAG_JOURNAL_FILE: u32 = 0x40000;

/// The inode of the root directory.
const ROOT_DIRECTORY_INODE: u32 = 2;
/// The root directory's default mode.
const ROOT_DIRECTORY_DEFAULT_MODE: u16 = INODE_PERMISSION_IRWXU
	| INODE_PERMISSION_IRGRP | INODE_PERMISSION_IXGRP
	| INODE_PERMISSION_IROTH | INODE_PERMISSION_IXOTH;

/// Directory entry type indicator: Unknown
const TYPE_INDICATOR_UNKNOWN: u8 = 0;
/// Directory entry type indicator: Regular file
const TYPE_INDICATOR_REGULAR: u8 = 1;
/// Directory entry type indicator: Directory
const TYPE_INDICATOR_DIRECTORY: u8 = 2;
/// Directory entry type indicator: Char device
const TYPE_INDICATOR_CHAR_DEVICE: u8 = 3;
/// Directory entry type indicator: Block device
const TYPE_INDICATOR_BLOCK_DEVICE: u8 = 4;
/// Directory entry type indicator: FIFO
const TYPE_INDICATOR_FIFO: u8 = 5;
/// Directory entry type indicator: Socket
const TYPE_INDICATOR_SOCKET: u8 = 6;
/// Directory entry type indicator: Symbolic link
const TYPE_INDICATOR_SYMLINK: u8 = 7;

/// Reads an object of the given type on the given device.
/// `offset` is the offset in bytes on the device.
/// `io` is the I/O interface of the device.
/// The function is marked unsafe because if the read object is invalid, the behaviour is
/// undefined.
unsafe fn read<T>(offset: u64, io: &mut dyn DeviceHandle) -> Result<T, Errno> {
	let size = size_of::<T>();
	let mut obj = MaybeUninit::<T>::uninit();

	let ptr = obj.as_mut_ptr() as *mut u8;
	let buffer = slice::from_raw_parts_mut(ptr, size);
	io.read(offset, buffer)?;

	Ok(obj.assume_init())
}

/// Writes an object of the given type on the given device.
/// `obj` is the object to write.
/// `offset` is the offset in bytes on the device.
/// `io` is the I/O interface of the device.
fn write<T>(obj: &T, offset: u64, io: &mut dyn DeviceHandle) -> Result<(), Errno> {
	let size = size_of_val(obj);
	let ptr = obj as *const T as *const u8;
	let buffer = unsafe {
		slice::from_raw_parts(ptr, size)
	};
	io.write(offset, buffer)?;

	Ok(())
}

/// Reads the `i`th block on the given device and writes the data onto the given buffer.
/// `i` is the offset of the block on the device.
/// `superblock` is the filesystem's superblock.
/// `io` is the I/O interface of the device.
/// `buff` is the buffer to write the data on.
/// If the block is outside of the storage's bounds, the function returns a error.
fn read_block(i: u64, superblock: &Superblock, io: &mut dyn DeviceHandle, buff: &mut [u8])
	-> Result<(), Errno> {
	let blk_size = superblock.get_block_size() as u64;
	io.read(i * blk_size, buff)?;

	Ok(())
}

// TODO Write block

/// The ext2 superblock structure.
#[repr(C, packed)]
struct Superblock {
	/// Total number of inodes in the filesystem.
	total_inodes: u32,
	/// Total number of blocks in the filesystem.
	total_blocks: u32,
	/// Number of blocks reserved for the superuser.
	superuser_blocks: u32,
	/// Total number of unallocated blocks.
	total_unallocated_blocks: u32,
	/// Total number of unallocated inodes.
	total_unallocated_inodes: u32,
	/// Block number of the block containing the superblock.
	superblock_block_number: u32,
	/// log2(block_size) - 10
	block_size_log: u32,
	/// log2(fragment_size) - 10
	fragment_size_log: u32,
	/// The number of blocks per block group.
	blocks_per_group: u32,
	/// The number of fragments per block group.
	fragments_per_group: u32,
	/// The number of inodes per block group.
	inodes_per_group: u32,
	/// The timestamp of the last mount operation.
	last_mount_timestamp: u32,
	/// The timestamp of the last write operation.
	last_write_timestamp: u32,
	/// The number of mounts since the last consistency check.
	mount_count_since_fsck: u16,
	/// The number of mounts allowed before a consistency check must be done.
	mount_count_before_fsck: u16,
	/// The ext2 signature.
	signature: u16,
	/// The filesystem's state.
	fs_state: u16,
	/// The action to perform when an error is detected.
	error_action: u16,
	/// The minor version.
	minor_version: u16,
	/// The timestamp of the last consistency check.
	last_fsck_timestamp: u32,
	/// The interval between mandatory consistency checks.
	fsck_interval: u32,
	/// The id os the operating system from which the filesystem was created.
	os_id: u32,
	/// The major version.
	major_version: u32,
	/// The UID of the user that can use reserved blocks.
	uid_reserved: u16,
	/// The GID of the group that can use reserved blocks.
	gid_reserved: u16,

	// Extended superblock fields

	/// The first non reserved inode
	first_non_reserved_inode: u32,
	/// The size of the inode structure in bytes.
	inode_size: u16,
	/// The block group containing the superblock.
	superblock_group: u16,
	/// Optional features for the implementation to support.
	optional_features: u32,
	/// Required features for the implementation to support.
	required_features: u32,
	/// Required features for the implementation to support for writing.
	write_required_features: u32,
	/// The filesystem id.
	filesystem_id: [u8; 16],
	/// The volume name.
	volume_name: [u8; 16],
	/// The path the volume was last mounted to.
	last_mount_path: [u8; 64],
	/// TODO doc
	compression_algorithms: u32,
	/// The number of blocks to preallocate for files.
	files_preallocate_count: u8,
	/// The number of blocks to preallocate for directories.
	direactories_preallocate_count: u8,
	/// Unused.
	_unused: u16,
	/// The journal ID.
	journal_id: [u8; 16],
	/// The journal inode.
	journal_inode: u32,
	/// The journal device.
	journal_device: u32,
	/// The head of orphan inodes list.
	orphan_inode_head: u32,

	/// Structure padding.
	_padding: [u8; 788],
}

impl Superblock {
	/// Creates a new instance by reading from the given device.
	pub fn read(io: &mut dyn DeviceHandle) -> Result<Self, Errno> {
		unsafe {
			read::<Self>(SUPERBLOCK_OFFSET, io)
		}
	}

	/// Tells whether the superblock is valid.
	pub fn is_valid(&self) -> bool {
		self.signature == EXT2_SIGNATURE
	}

	/// Returns the size of a block.
	pub fn get_block_size(&self) -> usize {
		math::pow2(self.block_size_log + 10) as _
	}

	/// Returns the size of a fragment.
	pub fn get_fragment_size(&self) -> usize {
		math::pow2(self.fragment_size_log + 10) as _
	}

	/// Writes the superblock on the device.
	pub fn write(&self, io: &mut dyn DeviceHandle) -> Result<(), Errno> {
		write::<Self>(self, SUPERBLOCK_OFFSET, io)
	}
}

/// TODO doc
#[repr(C, packed)]
struct BlockGroupDescriptor {
	/// The block address of the block usage bitmap.
	block_usage_bitmap_addr: u32,
	/// The block address of the inode usage bitmap.
	inode_usage_bitmap_addr: u32,
	/// Starting block address of inode table.
	inode_table_start_addr: u32,
	/// Number of unallocated blocks in group.
	unallocated_blocks_number: u16,
	/// Number of unallocated inodes in group.
	unallocated_inodes_number: u16,
	/// Number of directories in group.
	directories_number: u16,

	/// Structure padding.
	_padding: [u8; 14],
}

impl BlockGroupDescriptor {
	/// Reads the `i`th block group descriptor from the given device.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	pub fn read(i: u32, superblock: &Superblock, io: &mut dyn DeviceHandle)
		-> Result<Self, Errno> {
		let off = (superblock.get_block_size() * BGDT_BLOCK_OFFSET)
			+ (i as usize * size_of::<Self>());
		unsafe {
			read::<Self>(off as _, io)
		}
	}
}

/// TODO doc
#[repr(C, packed)]
struct Ext2INode {
	/// Type and permissions.
	mode: u16,
	/// User ID.
	uid: u16,
	/// Lower 32 bits of size in bytes.
	size_low: u32,
	/// Timestamp of the last modification of the metadata.
	ctime: u32,
	/// Timestamp of the last modification of the content.
	mtime: u32,
	/// Timestamp of the last access.
	atime: u32,
	/// Timestamp of the deletion.
	dtime: u32,
	/// Group ID.
	gid: u16,
	/// The number of hard links to this inode.
	hard_links_count: u16,
	/// The number of sectors used by this inode.
	used_sectors: u32,
	/// INode flags.
	flags: u32,
	/// OS-specific value.
	os_specific_0: u32,
	/// Direct block pointers.
	direct_block_ptrs: [u32; DIRECT_BLOCKS_COUNT],
	/// Simply indirect block pointer.
	singly_indirect_block_ptr: u32,
	/// Doubly indirect block pointer.
	doubly_indirect_block_ptr: u32,
	/// Triply indirect block pointer.
	triply_indirect_block_ptr: u32,
	/// Generation number.
	generation: u32,
	/// TODO doc
	extended_attributes_block: u32,
	/// Higher 32 bits of size in bytes.
	size_high: u32,
	/// Block address of fragment.
	fragment_addr: u32,
	/// OS-specific value.
	os_specific_1: [u8; 12],
}

impl Ext2INode {
	/// Returns the size of an inode.
	/// `superblock` is the filesystem's superblock.
	pub fn get_inode_size(superblock: &Superblock) -> usize {
		if superblock.major_version >= 1 {
			superblock.inode_size as _
		} else {
			128
		}
	}

	/// Returns the offset of the inode on the disk in bytes.
	/// `i` is the inode's index (starting at `1`).
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	fn get_disk_offset(i: u32, superblock: &Superblock, io: &mut dyn DeviceHandle)
		-> Result<u64, Errno> {
		let blk_size = superblock.get_block_size();
		let blk_grp = (i - 1) / superblock.inodes_per_group;
		let inode_off = (i - 1) % superblock.inodes_per_group;
		let inode_size = Self::get_inode_size(superblock);
		let inode_table_blk_off = (inode_off * inode_size as u32) / (blk_size as u32);

		let bgd = BlockGroupDescriptor::read(blk_grp, superblock, io)?;
		let inode_table_blk = bgd.inode_table_start_addr + inode_table_blk_off;
		Ok((inode_table_blk as u64 * blk_size as u64) + (inode_off as u64 * inode_size as u64))
	}

	/// Returns the mode for the given file `file`.
	fn get_file_mode(file: &File) -> u16 {
		let t = match file.get_file_type() {
			FileType::FIFO => INODE_TYPE_FIFO,
			FileType::CharDevice => INODE_TYPE_CHAR_DEVICE,
			FileType::Directory => INODE_TYPE_DIRECTORY,
			FileType::BlockDevice => INODE_TYPE_BLOCK_DEVICE,
			FileType::Regular => INODE_TYPE_REGULAR,
			FileType::Link => INODE_TYPE_SYMLINK,
			FileType::Socket => INODE_TYPE_SOCKET,
		};

		file.get_mode() | t
	}

	/// Reads the `i`th inode from the given device. The index `i` starts at `1`.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	pub fn read(i: u32, superblock: &Superblock, io: &mut dyn DeviceHandle)
		-> Result<Self, Errno> {
		let off = Self::get_disk_offset(i, superblock, io)?;
		unsafe {
			read::<Self>(off, io)
		}
	}

	/// Returns the type of the file.
	pub fn get_type(&self) -> FileType {
		let file_type = self.mode & 0xf000;

		match file_type {
			INODE_TYPE_FIFO => FileType::FIFO,
			INODE_TYPE_CHAR_DEVICE => FileType::CharDevice,
			INODE_TYPE_DIRECTORY => FileType::Directory,
			INODE_TYPE_BLOCK_DEVICE => FileType::BlockDevice,
			INODE_TYPE_REGULAR => FileType::Regular,
			INODE_TYPE_SYMLINK => FileType::Link,
			INODE_TYPE_SOCKET => FileType::Socket,

			_ => FileType::Regular,
		}
	}

	/// Returns the permissions of the file.
	pub fn get_permissions(&self) -> file::Mode {
		self.mode & 0x0fff
	}

	/// Returns the size of the file.
	/// `superblock` is the filesystem's superblock.
	pub fn get_size(&self, superblock: &Superblock) -> u64 {
		let has_version = superblock.major_version >= 1;
		let has_feature = superblock.write_required_features & WRITE_REQUIRED_64_BITS != 0;

		if has_version && has_feature {
			((self.size_high as u64) << 32) | (self.size_low as u64)
		} else {
			self.size_low as u64
		}
	}

	/// Resolves `n` block indirections to find the node's `i`th block, starting from block `blk`.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// If the block doesn't exist, the function returns None.
	fn resolve_indirections(n: usize, blk: u32, i: u32, superblock: &Superblock,
		io: &mut dyn DeviceHandle) -> Result<Option<u32>, Errno> {
		let blk_size = superblock.get_block_size();
		let entries_per_blk = blk_size / size_of::<u32>();

		let mut b = blk;
		for j in (0..n).rev() {
			let inner_off = i / ((j * entries_per_blk) as u32);
			let off = (blk as u64 * blk_size as u64) + (inner_off as u64);
			b = unsafe {
				read::<u32>(off, io)?
			};

			if b == 0 {
				break;
			}
		}

		Ok({
			if b != 0 {
				Some(b)
			} else {
				None
			}
		})
	}

	/// Returns the block offset of the node's content block with the given id `i`.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// If the block doesn't exist, the function returns None.
	fn get_content_block_off(&self, i: usize, superblock: &Superblock, io: &mut dyn DeviceHandle)
		-> Result<Option<u32>, Errno> {
		let blk_size = superblock.get_block_size();
		let entries_per_blk = blk_size / size_of::<u32>();

		if i < DIRECT_BLOCKS_COUNT {
			let blk = self.direct_block_ptrs[i];

			Ok({
				if blk != 0 {
					Some(blk as _)
				} else {
					None
				}
			})
		} else if i < DIRECT_BLOCKS_COUNT + entries_per_blk {
			let target = (i - DIRECT_BLOCKS_COUNT) as u32;
			Self::resolve_indirections(1, self.singly_indirect_block_ptr, target, superblock, io)
		} else if i < DIRECT_BLOCKS_COUNT + (entries_per_blk * entries_per_blk) {
			let target = (i - DIRECT_BLOCKS_COUNT - entries_per_blk) as u32;
			Self::resolve_indirections(2, self.singly_indirect_block_ptr, target, superblock, io)
		} else {
			let target = (i - DIRECT_BLOCKS_COUNT - (entries_per_blk * entries_per_blk)) as u32;
			Self::resolve_indirections(3, self.singly_indirect_block_ptr, target, superblock, io)
		}
	}

	/// Reads the content of the inode.
	/// `off` is the offset at which the inode is read.
	/// `buff` is the buffer in which the data is to be written.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	pub fn read_content(&self, off: u64, buff: &mut [u8], superblock: &Superblock,
		io: &mut dyn DeviceHandle) -> Result<(), Errno> {
		if off >= self.get_size(&superblock)
			|| off + buff.len() as u64 >= self.get_size(&superblock) {
			return Err(errno::EINVAL);
		}

		let blk_size = superblock.get_block_size();
		let begin_blk_id = (off / blk_size as u64) as usize;
		let end_blk_id = ((off + buff.len() as u64) / (blk_size as u64)) as usize;

		let mut blk_buff = malloc::Alloc::<u8>::new_default(blk_size)?;
		for i in begin_blk_id..end_blk_id {
			let blk_off = self.get_content_block_off(i as _, superblock, io).unwrap().unwrap();
			read_block(blk_off as _, superblock, io, blk_buff.get_slice_mut())?;

			// TODO Write content of blk_buff to buff
		}

		Ok(())
	}

	/// Writes the content of the inode.
	/// `off` is the offset at which the inode is written.
	/// `buff` is the buffer in which the data is to be read.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	pub fn write_content(&self, _off: usize, _buff: &mut [u8], _superblock: &Superblock,
		_io: &mut dyn DeviceHandle) -> Result<(), Errno> {
		// TODO

		Ok(())
	}

	/// Iterates over directory entries and calls the given function `f` for each.
	/// The function takes the offset of the entry in the inode and the entry itself.
	/// Free entries are also included.
	/// `f` returns a boolean telling whether the iteration may continue.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// If the file is not a directory, the behaviour is undefined.
	pub fn foreach_directory_entry<F: FnMut(usize, Box<DirectoryEntry>) -> bool>(&self, mut f: F,
		superblock: &Superblock, io: &mut dyn DeviceHandle) -> Result<(), Errno> {
		debug_assert_eq!(self.get_type(), FileType::Directory);

		let blk_size = superblock.get_block_size();
		let mut buff = malloc::Alloc::<u8>::new_default(blk_size)?;
		let mut i = 0;
		while i < self.get_size(superblock) {
			self.read_content(i, buff.get_slice_mut(), superblock, io)?;

			let mut j = 0;
			while j < blk_size {
				let entry = unsafe {
					DirectoryEntry::from(&mut buff.get_slice_mut()[j..])?
				};
				let total_size = entry.total_size as usize;

				if !f(j, entry) {
					return Ok(());
				}

				j += total_size;
			}
			debug_assert_eq!(j as u64, i + blk_size as u64);

			i += blk_size as u64;
		}

		Ok(())
	}

	/// Returns the directory entry with the given name `name`.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// If the entry doesn't exist, the function returns None.
	/// If the file is not a directory, the behaviour is undefined.
	pub fn get_directory_entry(&self, name: &str, superblock: &Superblock,
		io: &mut dyn DeviceHandle) -> Result<Option<Box<DirectoryEntry>>, Errno> {
		let mut entry = None;

		// TODO If the binary tree feature is enabled, use it
		self.foreach_directory_entry(| _, e | {
			if !e.is_free() && e.get_name(superblock) == name {
				entry = Some(e);
				false
			} else {
				true
			}
		}, superblock, io)?;

		Ok(entry)
	}

	/// Looks for a free entry in the inode.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// `min_size` is the minimum size of the entry in bytes.
	/// If the function finds an entry, it returns its offset. Else, the function returns None.
	fn get_free_entry(&self, superblock: &Superblock, io: &mut dyn DeviceHandle, min_size: usize)
		-> Result<Option<usize>, Errno> {
		let mut off_option = None;

		self.foreach_directory_entry(| off, e | {
			if e.is_free() && e.total_size as usize >= min_size {
				off_option = Some(off);
				false
			} else {
				true
			}
		}, superblock, io)?;

		Ok(off_option)
	}

	/// Adds a new entry to the current directory.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// `entry_inode` is the inode of the entry.
	/// `name` is the name of the entry.
	/// `file_type` is the type of the entry.
	/// If the block allocation fails or if the entry name is already used, the function returns an
	/// error.
	/// If the file is not a directory, the behaviour is undefined.
	pub fn add_dirent(&self, superblock: &Superblock, io: &mut dyn DeviceHandle,
		_entry_inode: u32, name: &String, _file_type: FileType) -> Result<(), Errno> {
		// TODO Ensure that the block size is large enough for every names?
		let blk_size = superblock.get_block_size();
		let entry_size = 8 + name.as_bytes().len();
		if entry_size > blk_size {
			return Err(errno::ENAMETOOLONG);
		}

		if let Some(free_entry_off) = self.get_free_entry(superblock, io, entry_size)? {
			let mut buff: [u8; 8] = [0; 8];
			self.read_content(free_entry_off as _, &mut buff, superblock, io)?;
			let free_entry = unsafe {
				DirectoryEntry::from(&buff)?
			};

			let split = free_entry.total_size as usize - entry_size > 8;
			if split {
				// TODO Split entry
			}

			// TODO Write the entry
		} else {
			// TODO Expand the size of the inode's content and add a new entry
		}

		Err(errno::ENOMEM)
	}

	// TODO remove_dirent

	/// Writes the inode on the device.
	pub fn write(&self, i: u32, superblock: &Superblock, io: &mut dyn DeviceHandle)
		-> Result<(), Errno> {
		let off = Self::get_disk_offset(i, superblock, io)?;
		write(self, off, io)
	}
}

/// A directory entry is a structure stored in the content of an inode of type Directory. Each
/// directory entry represent a file that is the stored in the directory and points to its inode.
#[repr(C, packed)]
struct DirectoryEntry {
	/// The inode associated with the entry.
	inode: u32,
	/// The total size of the entry.
	total_size: u16,
	/// Name length least-significant bits.
	name_length_lo: u8,
	/// Name length most-significant bits or type indicator (if enabled).
	name_length_hi: u8,
	/// The entry's name.
	name: [u8],
}

impl DirectoryEntry {
	/// Creates a new instance from a slice.
	pub unsafe fn from(slice: &[u8]) -> Result<Box<Self>, Errno> {
		let ptr = malloc::alloc(slice.len())? as *mut u8;
		let alloc_slice = slice::from_raw_parts_mut(ptr, slice.len());
		for i in 0..slice.len() {
			alloc_slice[i] = slice[i];
		}

		Ok(Box::from_raw(alloc_slice as *mut [u8] as *mut [()] as *mut Self))
	}

	/// Returns the length the entry's name.
	/// `superblock` is the filesystem's superblock.
	fn get_name_length(&self, superblock: &Superblock) -> usize {
		if superblock.required_features & REQUIRED_FEATURE_DIRECTORY_TYPE == 0 {
			((self.name_length_hi as usize) << 8) | (self.name_length_lo as usize)
		} else {
			self.name_length_lo as usize
		}
	}

	/// Returns the entry's name.
	/// `superblock` is the filesystem's superblock.
	pub fn get_name(&self, superblock: &Superblock) -> &str {
		let name_length = self.get_name_length(superblock);
		unsafe {
			util::ptr_to_str_len(&self.name[0], name_length)
		}
	}

	// TODO Function to set the name

	/// Tells whether the entry is valid.
	pub fn is_free(&self) -> bool {
		self.inode == 0
	}
}

/// Structure representing a instance of the ext2 filesystem.
struct Ext2Fs {
	/// The filesystem's superblock.
	superblock: Superblock,
}

impl Ext2Fs {
	/// Creates a new instance.
	/// If the filesystem cannot be mounted, the function returns an Err.
	fn new(mut superblock: Superblock, io: &mut dyn DeviceHandle) -> Result<Self, Errno> {
		debug_assert!(superblock.is_valid());

		// TODO Check that the driver supports required features
		let timestamp = time::get();
		if superblock.mount_count_since_fsck >= superblock.mount_count_before_fsck {
			return Err(errno::EINVAL);
		}
		if timestamp >= superblock.last_fsck_timestamp + superblock.fsck_interval {
			return Err(errno::EINVAL);
		}

		superblock.mount_count_since_fsck += 1;
		superblock.last_fsck_timestamp = timestamp;
		superblock.write(io)?;

		Ok(Self {
			superblock: superblock,
		})
	}

	/// Returns the number of block groups.
	fn get_block_groups_count(&self) -> usize {
		math::ceil_division(self.superblock.total_blocks,
			self.superblock.blocks_per_group) as _
	}

	/// Returns the id of a free inode in the filesystem.
	fn get_free_inode(&self) -> Result<u32, Errno> {
		// TODO
		Ok(0)
	}

	/// Marks the inode `inode` used on the filesystem.
	fn mark_inode_used(&self, _inode: u32) -> Result<(), Errno> {
		// TODO
		Ok(())
	}

	/// Marks the inode `inode` available on the filesystem.
	fn free_inode(&self, _inode: u32) -> Result<(), Errno> {
		// TODO
		Ok(())
	}
}

impl Filesystem for Ext2Fs {
	fn get_name(&self) -> &str {
		"ext2"
	}

	/// Tells whether the filesystem is mounted in read-only.
	fn is_readonly(&self) -> bool {
		// TODO Check that the driver supports write-required features
		false
	}

	fn get_inode(&mut self, io: &mut dyn DeviceHandle, path: Path) -> Result<INode, Errno> {
		debug_assert!(path.is_absolute());

		let mut inode_index = ROOT_DIRECTORY_INODE;
		for i in 0..path.get_elements_count() {
			let inode = Ext2INode::read(ROOT_DIRECTORY_INODE as _, &self.superblock, io)?;
			if inode.get_type() != FileType::Directory {
				return Err(errno::ENOENT);
			}

			let name = path[i].as_str();
			if let Some(entry) = inode.get_directory_entry(name, &self.superblock, io)? {
				inode_index = entry.inode;
			} else {
				return Err(errno::ENOENT);
			}
		}

		Ok(inode_index)
	}

	fn load_file(&mut self, io: &mut dyn DeviceHandle, inode: INode, name: String)
		-> Result<File, Errno> {
		let inode_ = Ext2INode::read(inode, &self.superblock, io)?;

		let mut file = File::new(name, inode_.get_type(), inode_.uid, inode_.gid,
			inode_.get_permissions())?;
		file.set_inode(Some(inode));
		file.set_ctime(inode_.ctime);
		file.set_mtime(inode_.mtime);
		file.set_atime(inode_.atime);

		Ok(file)
	}

	fn add_file(&mut self, io: &mut dyn DeviceHandle, parent_inode: INode, file: File)
		-> Result<(), Errno> {
		let parent = Ext2INode::read(parent_inode, &self.superblock, io)?;
		debug_assert_eq!(parent.get_type(), FileType::Directory);

		let inode_index = self.get_free_inode()?;
		let inode = Ext2INode {
			mode: Ext2INode::get_file_mode(&file),
			uid: file.get_uid(),
			size_low: 0,
			ctime: file.get_ctime(),
			mtime: file.get_mtime(),
			atime: file.get_atime(),
			dtime: 0,
			gid: file.get_gid(),
			hard_links_count: 1,
			used_sectors: 0,
			flags: 0,
			os_specific_0: 0,
			direct_block_ptrs: [0; DIRECT_BLOCKS_COUNT],
			singly_indirect_block_ptr: 0,
			doubly_indirect_block_ptr: 0,
			triply_indirect_block_ptr: 0,
			generation: 0,
			extended_attributes_block: 0,
			size_high: 0,
			fragment_addr: 0,
			os_specific_1: [0; 12],
		};
		inode.write(inode_index, &self.superblock, io)?;

		parent.add_dirent(&self.superblock, io, inode_index, file.get_name(),
			file.get_file_type())?;
		self.mark_inode_used(inode_index)
	}

	fn remove_file(&mut self, io: &mut dyn DeviceHandle, parent_inode: INode, _name: &String)
		-> Result<(), Errno> {
		let parent = Ext2INode::read(parent_inode, &self.superblock, io)?;
		debug_assert_eq!(parent.get_type(), FileType::Directory);

		// TODO

		Err(errno::ENOMEM)
	}

	fn read_node(&mut self, _io: &mut dyn DeviceHandle, _node: INode, _buf: &mut [u8])
		-> Result<(), Errno> {
		// TODO

		Err(errno::ENOMEM)
	}

	fn write_node(&mut self, _io: &mut dyn DeviceHandle, _node: INode, _buf: &[u8])
		-> Result<(), Errno> {
		// TODO

		Err(errno::ENOMEM)
	}
}

/// Structure representing the ext2 filesystem type.
pub struct Ext2FsType {}

impl FilesystemType for Ext2FsType {
	fn get_name(&self) -> &str {
		"ext2"
	}

	// TODO Also check partition type
	fn detect(&self, io: &mut dyn DeviceHandle) -> bool {
		if let Ok(superblock) = Superblock::read(io) {
			superblock.is_valid()
		} else {
			// TODO Return an error?
			false
		}
	}

	fn create_filesystem(&self, io: &mut dyn DeviceHandle) -> Result<Box<dyn Filesystem>, Errno> {
		let timestamp = time::get();
		let blocks_count = (io.get_size() / DEFAULT_BLOCK_SIZE) as u32;
		let groups_count = blocks_count / DEFAULT_BLOCKS_PER_GROUP;

		let inodes_count = groups_count * DEFAULT_INODES_PER_GROUP;

		let bgdt_off = BGDT_BLOCK_OFFSET * DEFAULT_BLOCK_SIZE as usize;
		let block_usage_bitmap_size = math::ceil_division(DEFAULT_BLOCKS_PER_GROUP,
			(DEFAULT_BLOCK_SIZE * 8) as _);
		let inode_usage_bitmap_size = math::ceil_division(DEFAULT_INODES_PER_GROUP,
			(DEFAULT_BLOCK_SIZE * 8) as _);
		let inode_table_size = math::ceil_division(DEFAULT_INODES_PER_GROUP * DEFAULT_INODE_SIZE as u32,
			(DEFAULT_BLOCK_SIZE * 8) as _);

		let available_blocks_per_group = DEFAULT_BLOCKS_PER_GROUP
			- (block_usage_bitmap_size + inode_usage_bitmap_size + inode_table_size);

		let superblock = Superblock {
			total_inodes: inodes_count,
			total_blocks: blocks_count,
			superuser_blocks: 0,
			total_unallocated_blocks: blocks_count,
			total_unallocated_inodes: inodes_count,
			superblock_block_number: (SUPERBLOCK_OFFSET / DEFAULT_BLOCK_SIZE) as _,
			block_size_log: (math::log2(DEFAULT_BLOCK_SIZE as usize) - 10) as _,
			fragment_size_log: 0, // TODO
			blocks_per_group: DEFAULT_BLOCKS_PER_GROUP,
			fragments_per_group: 0, // TODO
			inodes_per_group: DEFAULT_INODES_PER_GROUP,
			last_mount_timestamp: timestamp,
			last_write_timestamp: timestamp,
			mount_count_since_fsck: 0,
			mount_count_before_fsck: DEFAULT_MOUNT_COUNT_BEFORE_FSCK,
			signature: EXT2_SIGNATURE,
			fs_state: FS_STATE_CLEAN,
			error_action: ERR_ACTION_READ_ONLY,
			minor_version: DEFAULT_MINOR,
			last_fsck_timestamp: timestamp,
			fsck_interval: DEFAULT_FSCK_INTERVAL,
			os_id: 0xdeadbeef,
			major_version: DEFAULT_MAJOR,
			uid_reserved: 0,
			gid_reserved: 0,

			first_non_reserved_inode: 11,
			inode_size: DEFAULT_INODE_SIZE,
			superblock_group: ((SUPERBLOCK_OFFSET / DEFAULT_BLOCK_SIZE) as u32
				/ DEFAULT_BLOCKS_PER_GROUP) as _,
			optional_features: 0,
			required_features: 0,
			write_required_features: WRITE_REQUIRED_64_BITS,
			filesystem_id: [0; 16], // TODO
			volume_name: [0; 16], // TODO
			last_mount_path: [0; 64], // TODO
			compression_algorithms: 0,
			files_preallocate_count: 0,
			direactories_preallocate_count: 0,
			_unused: 0,
			journal_id: [0; 16], // TODO
			journal_inode: 0, // TODO
			journal_device: 0, // TODO
			orphan_inode_head: 0, // TODO

			_padding: [0; 788],
		};
		superblock.write(io)?;

		for i in 0..groups_count {
			// TODO Take into account the BGDT into the addresses/available blocks/inodes
			// TODO Add root directory
			let bgd = BlockGroupDescriptor {
				block_usage_bitmap_addr: i * DEFAULT_BLOCKS_PER_GROUP,
				inode_usage_bitmap_addr: i * DEFAULT_BLOCKS_PER_GROUP + block_usage_bitmap_size,
				inode_table_start_addr: i * DEFAULT_BLOCKS_PER_GROUP + block_usage_bitmap_size
					+ inode_usage_bitmap_size,
				unallocated_blocks_number: available_blocks_per_group as _,
				unallocated_inodes_number: DEFAULT_INODES_PER_GROUP as _,
				directories_number: 0,

				_padding: [0; 14],
			};

			let off = (bgdt_off + size_of::<BlockGroupDescriptor>() * i as usize) as u64;
			write(&bgd, off, io)?;
		}

		let root_dir = Ext2INode {
			mode: INODE_TYPE_DIRECTORY | ROOT_DIRECTORY_DEFAULT_MODE,
			uid: 0,
			size_low: 0,
			ctime: timestamp,
			mtime: timestamp,
			atime: timestamp,
			dtime: 0,
			gid: 0,
			hard_links_count: 1,
			used_sectors: 0,
			flags: 0,
			os_specific_0: 0,
			direct_block_ptrs: [0; DIRECT_BLOCKS_COUNT],
			singly_indirect_block_ptr: 0,
			doubly_indirect_block_ptr: 0,
			triply_indirect_block_ptr: 0,
			generation: 0,
			extended_attributes_block: 0,
			size_high: 0,
			fragment_addr: 0,
			os_specific_1: [0; 12],
		};
		root_dir.write(ROOT_DIRECTORY_INODE, &superblock, io)?;

		self.load_filesystem(io)
	}

	fn load_filesystem(&self, io: &mut dyn DeviceHandle) -> Result<Box<dyn Filesystem>, Errno> {
		let superblock = Superblock::read(io)?;
		Ok(Box::new(Ext2Fs::new(superblock, io)?)? as _)
	}
}