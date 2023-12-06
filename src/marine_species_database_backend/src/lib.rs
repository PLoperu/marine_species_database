#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use validator::Validate;
use std::{borrow::Cow, cell::RefCell};


type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct Taxonomy {
    id:u64,
    kingdom:String,
    phylum:String,
    class:String,
    order:String,
    family:String,
    genus:String,
    species:String,
    created_at:u64,
    updated_at:Option<u64>
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct MarineSpecie {
    id: u64,
    name: String,
    habitat:String,
    taxonomy_id: u64,//Reference to Taxonomy by Id
    conservation_status:String,  //Can be eg:Extinct,CriticallyEndagered,, Endagered, vulnerable,LeastConcern
    created_at:u64,
    updated_at:Option<u64>
}



// Implement Storable and BoundedStorable traits for Taxonomy
impl Storable for Taxonomy {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Taxonomy {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// Implement Storable and BoundedStorable traits for MarineSpecie
impl Storable for MarineSpecie {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for MarineSpecie {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static TAXONOMY_ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );
    static MARINESPECIE_ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))), 0)
            .expect("Cannot create a counter")
    );
    static TAXONOMY_STR: RefCell<StableBTreeMap<u64, Taxonomy, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
    ));

    static MARINESPECIE_STR: RefCell<StableBTreeMap<u64, MarineSpecie, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3)))
    ));

}

#[derive(candid::CandidType, Serialize, Deserialize, Default,Validate)]
struct TaxonomyPayload {
    #[validate(length(min = 1))]
    kingdom:String,
    #[validate(length(min = 1))]
    phylum:String,
    #[validate(length(min = 1))]
    class:String,
    #[validate(length(min = 1))]
    order:String,
    #[validate(length(min = 1))]
    family:String,
    #[validate(length(min = 1))]
    genus:String,
    #[validate(length(min = 1))]
    species:String,
}

#[derive(candid::CandidType, Serialize, Deserialize, Default,Validate)]
struct MarineSpeciePayload {
    #[validate(length(min = 1))]
    name: String,
    #[validate(length(min = 1))]
    habitat:String,
    #[validate(range(min = 0))]
    taxonomy_id: u64, //Reference to Taxonomy by ID
    #[validate(length(min = 1))]
    conservation_status:String,
}

// CRUD Implementation below 
#[ic_cdk::query]
fn get_all_taxonomy() -> Result<Vec<Taxonomy>, Error> {
    let taxonomy_map: Vec<(u64, Taxonomy)> =
        TAXONOMY_STR.with(|service| service.borrow().iter().collect());
    let taxonomys: Vec<Taxonomy> = taxonomy_map
        .into_iter()
        .map(|(_, taxonomy)| taxonomy)
        .collect();

    if !taxonomys.is_empty() {
        Ok(taxonomys)
    } else {
        Err(Error::NotFound {
            msg: "No Taxonomys found..... Empty! .".to_string(),
        })
    }
}
#[ic_cdk::query]
fn get_taxonomy(id: u64) -> Result<Taxonomy, Error> {
    match _get_taxonomy(&id) {
        Some(taxonomy) => Ok(taxonomy),
        None => Err(Error::NotFound {
            msg: format!("taxonomy with id={} not found", id),
        }),
    }
}



// Helper function 
fn _get_taxonomy(id: &u64) -> Option<Taxonomy> {
    TAXONOMY_STR.with(|service| service.borrow().get(id))
}

#[ic_cdk::update]
fn add_taxonomy(taxonomy_payload: TaxonomyPayload) -> Result<Taxonomy, Error> {
    let check_payload = taxonomy_payload.validate();
    if check_payload.is_err() {
        return Err(Error::ValidationFailed {
            content: check_payload.err().unwrap().to_string(),
        });
    }
    let id = TAXONOMY_ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");
    let taxonomy = Taxonomy { 
        id,
         kingdom: taxonomy_payload.kingdom,
         phylum: taxonomy_payload.phylum,
         class: taxonomy_payload.class,
         order: taxonomy_payload.order,
         family: taxonomy_payload.family,
         genus: taxonomy_payload.genus,
         species: taxonomy_payload.species,
        created_at: time(), 
        updated_at: None 
    } ;
    do_insert_taxonomy(&taxonomy);
    Ok(taxonomy)
}

#[ic_cdk::update]
fn update_taxonomy(id: u64, taxonomy_payload: TaxonomyPayload) -> Result<Taxonomy, Error> {
    let check_payload = taxonomy_payload.validate();
    if check_payload.is_err() {
        return Err(Error::ValidationFailed {
            content: check_payload.err().unwrap().to_string(),
        });
    }
    match TAXONOMY_STR.with(|service| service.borrow().get(&id)) {
        Some(mut taxonomy) => {
            taxonomy.kingdom = taxonomy_payload.kingdom;
            taxonomy.phylum =   taxonomy_payload.phylum;
            taxonomy.class =    taxonomy_payload.class;
            taxonomy.order =    taxonomy_payload.order;
            taxonomy.family =   taxonomy_payload.family;
            taxonomy.genus =    taxonomy_payload.genus;
            taxonomy.species =  taxonomy_payload.species;
            taxonomy.updated_at = Some(time());
            do_insert_taxonomy(&taxonomy);
            Ok(taxonomy)
        }
        None => Err(Error::NotFound {
            msg: format!(
                "Update Taxonomy  with id={}. not found",
                id
            ),
        }),
    }
}

// helper method to perform insert.
fn do_insert_taxonomy(taxonomy: &Taxonomy) {
    TAXONOMY_STR.with(|service| {
        service
            .borrow_mut()
            .insert(taxonomy.id, taxonomy.clone())
    });
}

#[ic_cdk::update]
fn delete_taxonomy(id: u64) -> Result<Taxonomy, Error> {
    match TAXONOMY_STR.with(|service| service.borrow_mut().remove(&id)) {
        Some(taxonomy) => Ok(taxonomy),
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't delete a taxonomy with id={}. taxonomy not found.",
                id
            ),
        }),
    }
}


// Marine Specie 

#[ic_cdk::query]
fn get_all_marinespecie() -> Result<Vec<MarineSpecie>, Error> {
    let marinespecie_map: Vec<(u64, MarineSpecie)> =
        MARINESPECIE_STR.with(|service| service.borrow().iter().collect());
    let marinespecies: Vec<MarineSpecie> = marinespecie_map.into_iter().map(|(_, marinespecie)| marinespecie).collect();

    if !marinespecies.is_empty() {
        Ok(marinespecies)
    } else {
        Err(Error::NotFound {
            msg: "No marinespecie found.".to_string(),
        })
    }
}


#[ic_cdk::query]
fn get_marinespecie(id: u64) -> Result<MarineSpecie, Error> {
    match _get_marinespecie(&id) {
        Some(marinespecie) => Ok(marinespecie),
        None => Err(Error::NotFound {
            msg: format!("Marine_Specie with id={} not found", id),
        }),
    }
}
// a helper method to get a marine specie by id.
fn _get_marinespecie(id: &u64) -> Option<MarineSpecie> {
    MARINESPECIE_STR.with(|service| service.borrow().get(id))
}



// Get marine specie by conservation_status
#[ic_cdk::query]
fn get_marinespecie_by_conservation_status(conservation_status: String) -> Result<Vec<MarineSpecie>, Error> {
    let marinespecie_map: Vec<(u64, MarineSpecie)> =
        MARINESPECIE_STR.with(|service| service.borrow().iter().collect());

    // Filter by conservation status
    let marinespecie_in_conservation_status: Vec<MarineSpecie> = marinespecie_map
        .into_iter()
        .map(|(_, marinespecie)| marinespecie)
        .filter(|marinespecie| marinespecie.conservation_status.to_lowercase() == conservation_status.to_lowercase())
        .collect();

    if !marinespecie_in_conservation_status.is_empty() {
        Ok(marinespecie_in_conservation_status)
    } else {
        Err(Error::NotFound {
            msg: format!("No Marine Specie  found in classified conservation_status: {}", conservation_status),
        })
    }
}


#[ic_cdk::update]
fn add_marinespecie(marinespecie_payload: MarineSpeciePayload) -> Result<MarineSpecie, Error> {
    let check_payload = marinespecie_payload.validate();
    if check_payload.is_err() {
        return Err(Error::ValidationFailed {
            content: check_payload.err().unwrap().to_string(),
        });
    }
    let id = MARINESPECIE_ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");

    let marinespecie = MarineSpecie { 
        id,
        name: marinespecie_payload.name,
        habitat: marinespecie_payload.habitat,
        taxonomy_id: marinespecie_payload.taxonomy_id,
        conservation_status: marinespecie_payload.conservation_status,
        created_at: time(),
        updated_at: None
    };
    do_insert_marinespecie(&marinespecie);
    Ok(marinespecie)
}

#[ic_cdk::update]
fn update_marinespecie(id: u64, marinespecie_payload: MarineSpeciePayload) -> Result<MarineSpecie, Error> {
    let check_payload = marinespecie_payload.validate();
    if check_payload.is_err() {
        return Err(Error::ValidationFailed {
            content: check_payload.err().unwrap().to_string(),
        });
    }
    match MARINESPECIE_STR.with(|service| service.borrow().get(&id)) {
        Some(mut marinespecie) => {
            marinespecie.name = marinespecie_payload.name;
            marinespecie.habitat = marinespecie_payload.habitat;
            marinespecie.taxonomy_id = marinespecie_payload.taxonomy_id;
            marinespecie.conservation_status = marinespecie_payload.conservation_status;
            marinespecie.updated_at = Some(time());
            do_insert_marinespecie(&marinespecie);
            Ok(marinespecie)
        }
        None => Err(Error::NotFound {
            msg: format!("could not update Marine Specie with id={}.", id),
        }),
    }
}

// helper method to perform insert.
fn do_insert_marinespecie(marinespecie: &MarineSpecie) {
    MARINESPECIE_STR.with(|service| service.borrow_mut().insert(marinespecie.id, marinespecie.clone()));
}
#[ic_cdk::update]
fn delete_marinespecie(id: u64) -> Result<MarineSpecie, Error> {
    match MARINESPECIE_STR.with(|service| service.borrow_mut().remove(&id)) {
        Some(marinespecie) => Ok(marinespecie),
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't delete a marinespecie with id={}.",
                id
            ),
        }),
    }
}
#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
    ValidationFailed { content: String },
    InvalidInput 
}

// need this to generate candid
ic_cdk::export_candid!();