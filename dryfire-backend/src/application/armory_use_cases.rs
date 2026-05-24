use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    application::app_state::AppState,
    domain::{
        entities::armory::{
            AmmoLot, AmmoTransaction, AmmoTxnKind, BulletType, Caliber, Gun,
            ShellType, WeaponClass,
        },
        errors::{DomainError, DomainResult, ValidationError},
        repositories::armory::PageQuery,
    },
    utils::b64::b64_encode_bytes,
};

#[derive(Debug)]
pub struct CreateGunInput {
    pub owner_id: Uuid,
    pub manufacturer: String,
    pub model: String,
    pub serial: String,
    pub class: String,
    pub caliber: String,
    pub date_of_purchase: DateTime<Utc>,
    pub photo_url: Option<String>,
    pub notes: Option<String>,
}

pub struct CreateGunUseCase<'a> { pub state: &'a AppState }
impl<'a> CreateGunUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }

    #[tracing::instrument(skip(self, input), fields(owner = %input.owner_id))]
    pub async fn execute(&self, input: CreateGunInput) -> DomainResult<Uuid> {
        let class: WeaponClass = input.class.parse()?;
        let caliber = Caliber::parse(input.caliber)?;
        let gun = Gun::register(
            input.owner_id, input.manufacturer, input.model, input.serial.clone(),
            class, caliber, input.date_of_purchase, input.photo_url, input.notes,
        )?;

        // Searchable HMAC of the serial (constant-time-comparable in SQL).
        let serial_hmac = b64_encode_bytes(&self.state.signer.sign(gun.serial()));
        // Placeholder for AEAD ciphertext — swap in a real Encryptor.
        let serial_cipher = base64_url::encode(gun.serial());

        self.state.gun_repo.create(&gun, &serial_cipher, &serial_hmac).await?;
        Ok(gun.id())
    }
}

pub struct GetGunUseCase<'a> { pub state: &'a AppState }
impl<'a> GetGunUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(&self, id: Uuid, owner_id: Uuid) -> DomainResult<Gun> {
        self.state.gun_repo.find_by_id(id, owner_id).await?
            .ok_or(DomainError::GunNotFound)
    }
}

pub struct ListGunsUseCase<'a> { pub state: &'a AppState }
impl<'a> ListGunsUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(&self, owner_id: Uuid, page: PageQuery)
        -> DomainResult<(Vec<Gun>, i64)>
    {
        self.state.gun_repo.list_for_owner(owner_id, page).await
    }
}

pub struct DeleteGunUseCase<'a> { pub state: &'a AppState }
impl<'a> DeleteGunUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(&self, id: Uuid, owner_id: Uuid) -> DomainResult<()> {
        self.state.gun_repo.soft_delete(id, owner_id).await
    }
}

// --- Ammo lots --- //
#[derive(Debug)]
pub struct CreateAmmoLotInput {
    pub owner_id: Uuid,
    pub manufacturer: String,
    pub caliber: String,
    pub bullet_type: String,
    pub shell_type: String,
    pub bullet_weight_grains: Option<f64>,
    pub powder_charge_grains: Option<f64>,
    pub initial_quantity: i64,
    pub notes: Option<String>,
}

pub struct CreateAmmoLotUseCase<'a> { pub state: &'a AppState }
impl<'a> CreateAmmoLotUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }

    pub async fn execute(&self, input: CreateAmmoLotInput) -> DomainResult<Uuid> {
        if input.initial_quantity < 0 {
            return Err(DomainError::Validation(ValidationError::Custom(
                "initial_quantity must be >= 0".into(),
            )));
        }
        let lot = AmmoLot {
            id: Uuid::new_v4(),
            owner_id: input.owner_id,
            manufacturer: input.manufacturer,
            caliber: Caliber::parse(input.caliber)?,
            bullet_type: input.bullet_type.parse::<BulletType>()?,
            shell_type:  input.shell_type.parse::<ShellType>()?,
            bullet_weight_grains: input.bullet_weight_grains,
            powder_charge_grains: input.powder_charge_grains,
            quantity_on_hand: input.initial_quantity,
            notes: input.notes,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.state.ammo_repo.create_lot(&lot).await?;
        Ok(lot.id)
    }
}

#[derive(Debug)]
pub struct RecordAmmoTxnInput {
    pub owner_id: Uuid,
    pub lot_id: Uuid,
    pub gun_id: Option<Uuid>,
    pub kind: String,
    pub quantity: i64,         // positive in the request
    pub happened_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

pub struct RecordAmmoTxnUseCase<'a> { pub state: &'a AppState }
impl<'a> RecordAmmoTxnUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }

    #[tracing::instrument(skip(self, input))]
    pub async fn execute(&self, input: RecordAmmoTxnInput) -> DomainResult<Uuid> {
        let kind: AmmoTxnKind = input.kind.parse()?;
        if input.quantity <= 0 {
            return Err(DomainError::Validation(ValidationError::Custom(
                "quantity must be > 0".into(),
            )));
        }
        let signed = match kind {
            AmmoTxnKind::Purchase | AmmoTxnKind::Adjust =>  input.quantity,
            AmmoTxnKind::Use      | AmmoTxnKind::Loss   => -input.quantity,
        };
        let txn = AmmoTransaction {
            id: Uuid::new_v4(),
            owner_id: input.owner_id,
            lot_id: input.lot_id,
            gun_id: input.gun_id,
            kind,
            delta: signed,
            happened_at: input.happened_at.unwrap_or_else(Utc::now),
            notes: input.notes,
            created_at: Utc::now(),
        };
        self.state.ammo_repo.record_txn(&txn).await?;
        Ok(txn.id)
    }
}

#[derive(Debug)]
pub struct AmmoStatsInput {
    pub owner_id: Uuid,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Debug, serde::Serialize)]
pub struct AmmoStats {
    pub by_caliber: Vec<(String, i64)>,
    pub by_gun: Vec<(Option<Uuid>, i64)>,
}

pub struct AmmoStatsUseCase<'a> { pub state: &'a AppState }
impl<'a> AmmoStatsUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(&self, input: AmmoStatsInput) -> DomainResult<AmmoStats> {
        let by_caliber = self.state.ammo_repo
            .usage_by_caliber(input.owner_id, input.from, input.to).await?;
        let by_gun = self.state.ammo_repo
            .usage_by_gun(input.owner_id, input.from, input.to).await?;
        Ok(AmmoStats { by_caliber, by_gun })
    }
}