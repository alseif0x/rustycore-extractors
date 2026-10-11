// SPDX-License-Identifier: GPL-3.0-or-later
//! Exact extraction catalog at the pinned C++ source. Only fourteen entries
//! have a source-backed float schema; six remain opaque complete raw files.
//! Metadata facts are original transcriptions, not imported implementation.
use super::{
    Error, Limits, MAX_DATA_ROWS, MAX_FILE_BYTES, MAX_VALUES, NumericTable, parse_columns,
};

pub const SOURCE_FACTS: &str = include_str!("../../../../contracts/forever-gt-catalog-v1.json");
pub const MAX_FILES: usize = 20;
pub const MAX_BATCH_BYTES: usize = MAX_FILES * MAX_FILE_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Table {
    ArtifactKnowledgeMultiplier,
    ArtifactLevelXP,
    BarberShopCostBase,
    BaseMp,
    BaseProfessionRatings,
    BattlePetTypeDamageMod,
    BattlePetXP,
    CombatRatings,
    CombatRatingsMultByILvl,
    HonorLevel,
    HpPerSta,
    ItemLevelByLevel,
    ItemLevelSquish,
    ItemSocketCostPerLevel,
    NPCManaCostScaler,
    ProfessionRatings,
    SandboxScaling,
    SpellScaling,
    StaminaMultByILvl,
    Xp,
}

struct Metadata {
    name: &'static str,
    file_id: u32,
    file_name: &'static str,
    path: &'static str,
    columns: Option<usize>,
}

const METADATA: [Metadata; MAX_FILES] = [
    Metadata {
        name: "ArtifactKnowledgeMultiplier",
        file_id: 1582086,
        file_name: "ArtifactKnowledgeMultiplier.txt",
        path: "gt/ArtifactKnowledgeMultiplier.txt",
        columns: Some(1),
    },
    Metadata {
        name: "ArtifactLevelXP",
        file_id: 1391662,
        file_name: "ArtifactLevelXP.txt",
        path: "gt/ArtifactLevelXP.txt",
        columns: Some(2),
    },
    Metadata {
        name: "BarberShopCostBase",
        file_id: 1391663,
        file_name: "BarberShopCostBase.txt",
        path: "gt/BarberShopCostBase.txt",
        columns: Some(1),
    },
    Metadata {
        name: "BaseMp",
        file_id: 1391664,
        file_name: "BaseMp.txt",
        path: "gt/BaseMp.txt",
        columns: Some(15),
    },
    Metadata {
        name: "BaseProfessionRatings",
        file_id: 4494528,
        file_name: "BaseProfessionRatings.txt",
        path: "gt/BaseProfessionRatings.txt",
        columns: None,
    },
    Metadata {
        name: "BattlePetTypeDamageMod",
        file_id: 1391665,
        file_name: "BattlePetTypeDamageMod.txt",
        path: "gt/BattlePetTypeDamageMod.txt",
        columns: None,
    },
    Metadata {
        name: "BattlePetXP",
        file_id: 1391666,
        file_name: "BattlePetXP.txt",
        path: "gt/BattlePetXP.txt",
        columns: Some(2),
    },
    Metadata {
        name: "CombatRatings",
        file_id: 1391669,
        file_name: "CombatRatings.txt",
        path: "gt/CombatRatings.txt",
        columns: Some(32),
    },
    Metadata {
        name: "CombatRatingsMultByILvl",
        file_id: 1391670,
        file_name: "CombatRatingsMultByILvl.txt",
        path: "gt/CombatRatingsMultByILvl.txt",
        columns: Some(4),
    },
    Metadata {
        name: "HonorLevel",
        file_id: 1391671,
        file_name: "HonorLevel.txt",
        path: "gt/HonorLevel.txt",
        columns: None,
    },
    Metadata {
        name: "HpPerSta",
        file_id: 1391642,
        file_name: "HpPerSta.txt",
        path: "gt/HpPerSta.txt",
        columns: Some(1),
    },
    Metadata {
        name: "ItemLevelByLevel",
        file_id: 2012881,
        file_name: "ItemLevelByLevel.txt",
        path: "gt/ItemLevelByLevel.txt",
        columns: Some(1),
    },
    Metadata {
        name: "ItemLevelSquish",
        file_id: 1726830,
        file_name: "ItemLevelSquish.txt",
        path: "gt/ItemLevelSquish.txt",
        columns: None,
    },
    Metadata {
        name: "ItemSocketCostPerLevel",
        file_id: 1391643,
        file_name: "ItemSocketCostPerLevel.txt",
        path: "gt/ItemSocketCostPerLevel.txt",
        columns: Some(1),
    },
    Metadata {
        name: "NPCManaCostScaler",
        file_id: 1391651,
        file_name: "NPCManaCostScaler.txt",
        path: "gt/NPCManaCostScaler.txt",
        columns: Some(1),
    },
    Metadata {
        name: "ProfessionRatings",
        file_id: 4492239,
        file_name: "ProfessionRatings.txt",
        path: "gt/ProfessionRatings.txt",
        columns: None,
    },
    Metadata {
        name: "SandboxScaling",
        file_id: 1391659,
        file_name: "SandboxScaling.txt",
        path: "gt/SandboxScaling.txt",
        columns: None,
    },
    Metadata {
        name: "SpellScaling",
        file_id: 1391660,
        file_name: "SpellScaling.txt",
        path: "gt/SpellScaling.txt",
        columns: Some(24),
    },
    Metadata {
        name: "StaminaMultByILvl",
        file_id: 1980632,
        file_name: "StaminaMultByILvl.txt",
        path: "gt/StaminaMultByILvl.txt",
        columns: Some(4),
    },
    Metadata {
        name: "xp",
        file_id: 1391661,
        file_name: "xp.txt",
        path: "gt/xp.txt",
        columns: Some(5),
    },
];

impl Table {
    /// Pin order from System.cpp, not the six-table v1 group order.
    pub const ALL: [Self; MAX_FILES] = [
        Self::ArtifactKnowledgeMultiplier,
        Self::ArtifactLevelXP,
        Self::BarberShopCostBase,
        Self::BaseMp,
        Self::BaseProfessionRatings,
        Self::BattlePetTypeDamageMod,
        Self::BattlePetXP,
        Self::CombatRatings,
        Self::CombatRatingsMultByILvl,
        Self::HonorLevel,
        Self::HpPerSta,
        Self::ItemLevelByLevel,
        Self::ItemLevelSquish,
        Self::ItemSocketCostPerLevel,
        Self::NPCManaCostScaler,
        Self::ProfessionRatings,
        Self::SandboxScaling,
        Self::SpellScaling,
        Self::StaminaMultByILvl,
        Self::Xp,
    ];
    pub const fn name(self) -> &'static str {
        METADATA[self as usize].name
    }
    pub const fn file_id(self) -> u32 {
        METADATA[self as usize].file_id
    }
    pub const fn file_name(self) -> &'static str {
        METADATA[self as usize].file_name
    }
    pub const fn relative_path(self) -> &'static str {
        METADATA[self as usize].path
    }
    pub const fn value_columns(self) -> Option<usize> {
        METADATA[self as usize].columns
    }
    pub const fn validation_mode(self) -> &'static str {
        if self.value_columns().is_some() {
            "f32"
        } else {
            "raw"
        }
    }
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|table| table.name() == name)
    }
}

/// Binds artifact identities/modes/counts to the reviewed source-facts file.
pub fn fingerprint() -> String {
    crate::sha256::hex(SOURCE_FACTS.as_bytes())
}

/// Admit only a source-backed numeric schema. Unknown entries cannot obtain
/// numeric facts even when their bytes happen to resemble a float table.
pub fn parse_numeric(bytes: &[u8], table: Table) -> Result<NumericTable, Error> {
    let columns = table.value_columns().ok_or(Error::Schema)?;
    parse_columns(
        bytes,
        columns,
        Limits {
            bytes: MAX_FILE_BYTES,
            data_rows: MAX_DATA_ROWS,
            values: MAX_VALUES,
        },
    )
}
