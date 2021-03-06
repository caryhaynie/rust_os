// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/metadevs/storage.rs
// - Storage (block device) subsystem
module_define!(Storage, [], init)

/// A unique handle to a storage volume (logical)
pub struct VolumeHandle
{
	lv_idx: uint,
}

/// Physical volume instance provided by driver
pub trait PhysicalVolume
{
	fn name(&self) -> &str;	// Local lifetime string
	fn blocksize(&self) -> uint;	// Must return a power of two
	fn capacity(&self) -> u64;
	fn read(&self, prio: uint, blockidx: u64, count: uint, dst: &mut [u8]) -> bool;
	fn write(&mut self, prio: uint, blockidx: u64, count: uint, src: &[u8]) -> bool;
	fn wipe(&mut self, blockidx: u64, count: uint);
}

/// Registration for a physical volume handling driver
trait Mapper
{
	fn name(&self) -> &str;
	fn handles_pv(&self, pv: &PhysicalVolume) -> uint;
}

/// A single logical volume, composed of 1 or more physical blocks
struct LogicalVolume
{
	block_size: uint,	///< Logical block size (max physical block size)
	region_size: Option<uint>,	///< Number of bytes in each physical region, None = JBOD
	regions: Vec<PhysicalRegion>,
}
/// Physical region used by a logical volume
struct PhysicalRegion
{
	volume: uint,
	block_count: uint,	// uint to save space in average case
	first_block: u64,
}

static s_next_pv_idx: AtomicUInt = ATOMIC_UINT_INIT;
static s_physical_volumes: Mutex<HashMap<uint,Box<PhysicalVolume+Send>>> = mutex_init!( hashmap_init!() );
static s_logical_volumes: Mutex<HashMap<uint,LogicalVolume>> = mutex_init!( hashmap_init!() );
static s_mappers: Mutex<Vec<&'static Mapper+Send> = mutex_init!( vec_init!() );

// TODO: Maintain a set of registered volumes. Mappers can bind onto a volume and register new LVs
// TODO: Maintain set of active mappings (set of PVs -> set of LVs)
// NOTE: Should unbinding of LVs be allowed? (Yes, for volume removal)

fn init()
{
}

pub fn register_pv(pv: Box<PhysicalVolume+Send>) -> PhysicalVolumeReg
{
	let pv_id = s_next_pv_idx.inc();
	s_physical_volumes.lock().set(pv_id, pv)

	// Now that a new PV has been inserted, handlers should be informed
	let mut best_mapper = None;
	let mut best_mapper_level = 0;
	let mappers = s_mappers.lock();
	for mapper in mappers.iter()
	{
		let level = mapper.handles_pv(&*pv);
		if level == 0
		{
		}
		else if level < best_mapper_level
		{
		}
		else if level == best_mapper_level
		{
			// Fight!
			log_warning!("LV Mappers {} and {} are fighting over {}",
				mapper.name(), best_mapper.unwrap().name(), pv.name());
		}
		else
		{
			best_mapper = Some(mapper);
			best_mapper_level = level;
		}
	}
	if let Some(mapper) = best_mapper
	{
		// Poke mapper
	}
	PhysicalVolumeReg { idx: pv_id }
}

/// Function called when a new volume is registered (physical or logical)
fn new_volume(volidx: uint)
{
}

pub fn enum_pvs() -> Vec<(uint,String)>
{
	s_physical_volumes.lock().iter().map(|k,v| (*k, String::new(*v)) ).collect()
}

// vim: ft=rust
