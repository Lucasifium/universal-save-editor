//! Pillars of Eternity `MobileObjects.save` (inside the `.savegame` zip, or on its own from the
//! Microsoft Store version): a run of SharpSerializer binary streams (the size-optimised
//! format). The first stream holds the number of objects; each object after it is a stream of
//! its own with its own tables of names and types.
//!
//! A value whose type is the one its field declares is written without a type, so its size
//! comes from the game's class definitions: `FIELDS` lists the declared type of every field the
//! saves use. The reading follows Eternity Keeper and its class definitions (GPL-3.0, see
//! THIRD_PARTY_NOTICES.md). Values are patched in place: the streams have no outer lengths or
//! checksums, so the rest of the file is written back exactly.

use crate::tree::{child, int_in, unique_key, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, OnceLock};

pub const KIND: &str = "Pillars of Eternity save";

/// How a simple value is stored.
#[derive(Clone, Copy, Debug, PartialEq)]
enum S {
    Bool,
    U8,
    I8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    Char,
    Dec,
    Str,
    Guid,
    Bytes,
}

/// A field's declared type: a simple value, or a class whose own fields are looked up.
#[derive(Clone, Copy, Debug)]
enum F {
    Simple(S),
    Class(&'static str),
}

/// (class, field, declared type), from the game's classes as Eternity Keeper defines them.
/// Nested classes are named by their own name; the .NET collections only by their `Capacity`.
#[rustfmt::skip]
const FIELDS: &[(&str, &str, F)] = &[
    ("AbilityMod", "EquipmentOrigin", F::Class("Equippable")),
    ("AbilityMod", "ReplaceObjects", F::Class("ReplaceObjectParams")),
    ("AbilityMod", "SourceType", F::Simple(S::I32)),
    ("AbilityMod", "StatusEffects", F::Class("StatusEffectParams")),
    ("AbilityMod", "Type", F::Simple(S::I32)),
    ("AbilityMod", "Value", F::Simple(S::F32)),
    ("Affliction", "Description", F::Class("DatabaseString")),
    ("Affliction", "DisengageAll", F::Simple(S::Bool)),
    ("Affliction", "DisplayName", F::Class("DatabaseString")),
    ("Affliction", "Exclusive", F::Simple(S::Bool)),
    ("Affliction", "Icon", F::Class("Texture2D")),
    ("Affliction", "Material", F::Class("MaterialReplacement")),
    ("Affliction", "Overrides", F::Class("Affliction")),
    ("Affliction", "StatusEffects", F::Class("StatusEffectParams")),
    ("Affliction", "ThinUI", F::Simple(S::Bool)),
    ("Affliction", "tag", F::Simple(S::Str)),
    ("AppearancePiece", "armorType", F::Simple(S::I32)),
    ("AppearancePiece", "bodyPiece", F::Simple(S::I32)),
    ("AppearancePiece", "hideBoots", F::Simple(S::Bool)),
    ("AppearancePiece", "hideFacialHair", F::Simple(S::Bool)),
    ("AppearancePiece", "hideGloves", F::Simple(S::Bool)),
    ("AppearancePiece", "hideHair", F::Simple(S::Bool)),
    ("AppearancePiece", "hideHead", F::Simple(S::Bool)),
    ("AppearancePiece", "materialVariation", F::Simple(S::I32)),
    ("AppearancePiece", "modelVariation", F::Simple(S::I32)),
    ("AppearancePiece", "partialFacialHair", F::Simple(S::Bool)),
    ("AppearancePiece", "partialHair", F::Simple(S::Bool)),
    ("AppearancePiece", "specialOverride", F::Simple(S::Str)),
    ("CharacterDatabaseString", "CharacterGuid", F::Simple(S::Str)),
    ("CharacterDatabaseString", "CharacterGuidSerialized", F::Simple(S::Str)),
    ("CharacterDatabaseString", "PackageSerialized", F::Simple(S::U32)),
    ("CharacterDatabaseString", "StringIDSerialized", F::Simple(S::I32)),
    ("CharacterDatabaseString", "StringTableID", F::Simple(S::I32)),
    ("CharacterDatabaseString", "StringTableSerialized", F::Simple(S::I32)),
    ("Color", "a", F::Simple(S::F32)),
    ("Color", "b", F::Simple(S::F32)),
    ("Color", "g", F::Simple(S::F32)),
    ("Color", "r", F::Simple(S::F32)),
    ("ComponentPersistencePacket", "TypeString", F::Simple(S::Str)),
    ("ComponentPersistencePacket", "Variables", F::Class("HashMap")),
    ("Container", "AdditionalInteractionObjects", F::Class("GameObject")),
    ("Container", "AlliesAttackThief", F::Simple(S::Bool)),
    ("Container", "Animation", F::Simple(S::I32)),
    ("Container", "AreaLootable", F::Simple(S::Bool)),
    ("Container", "ArrivalDistance", F::Simple(S::F32)),
    ("Container", "ArrivalRadius", F::Simple(S::F32)),
    ("Container", "AttackThief", F::Simple(S::Bool)),
    ("Container", "CanAreaLootContainer", F::Simple(S::Bool)),
    ("Container", "CurrentState", F::Simple(S::I32)),
    ("Container", "DeleteMeIfEmpty", F::Simple(S::Bool)),
    ("Container", "HasInteracted", F::Simple(S::Bool)),
    ("Container", "HasInteractionObject", F::Simple(S::Bool)),
    ("Container", "InteractionObject", F::Class("GameObject")),
    ("Container", "IsEmpty", F::Simple(S::Bool)),
    ("Container", "IsOwned", F::Simple(S::Bool)),
    ("Container", "IsRevealed", F::Simple(S::Bool)),
    ("Container", "IsUsable", F::Simple(S::Bool)),
    ("Container", "IsVisible", F::Simple(S::Bool)),
    ("Container", "Key", F::Class("Item")),
    ("Container", "LabelName", F::Class("DatabaseString")),
    ("Container", "LockDifficulty", F::Simple(S::I32)),
    ("Container", "LockPickItem", F::Class("Item")),
    ("Container", "ManualLabelName", F::Simple(S::Str)),
    ("Container", "MustHaveKey", F::Simple(S::Bool)),
    ("Container", "NotEmptyVFX", F::Class("GameObject")),
    ("Container", "PlayerHasBeenCaughtStealingFrom", F::Simple(S::Bool)),
    ("Container", "RemoveUsedKey", F::Simple(S::Bool)),
    ("Container", "StartingState", F::Simple(S::I32)),
    ("Container", "StealingFactionAdjustment", F::Simple(S::I32)),
    ("Container", "StealingFactionID", F::Simple(S::I32)),
    ("Container", "UsableRadius", F::Simple(S::F32)),
    ("Container", "UseRadius", F::Simple(S::F32)),
    ("Container", "User", F::Class("GameObject")),
    ("Container", "fireItemTakenEventOnlyOnce", F::Simple(S::Bool)),
    ("CurrencyValue", "v", F::Simple(S::F32)),
    ("DatabaseString", "PackageSerialized", F::Simple(S::U32)),
    ("DatabaseString", "StringIDSerialized", F::Simple(S::I32)),
    ("DatabaseString", "StringTableID", F::Simple(S::I32)),
    ("DatabaseString", "StringTableSerialized", F::Simple(S::I32)),
    ("DetectionObject", "Object", F::Simple(S::Guid)),
    ("DetectionObject", "Time", F::Simple(S::F32)),
    ("DetectionObject", "m_obj", F::Simple(S::Guid)),
    ("DetectionObject", "m_time", F::Simple(S::F32)),
    ("Dictionary", "Capacity", F::Simple(S::I32)),
    ("Disposition", "m_dispositions", F::Class("Integer")),
    ("Disposition", "m_pointsPerRank", F::Class("List")),
    ("EffectData", "m_durationMult", F::Simple(S::F32)),
    ("EffectData", "m_effectID", F::Simple(S::U32)),
    ("EffectData", "m_target", F::Class("GameObject")),
    ("EternityDateTime", "TotalSeconds", F::Simple(S::I32)),
    ("EternityTimeInterval", "SerializedSeconds", F::Simple(S::I32)),
    ("ExperienceSerializerPacket", "ExperienceGiven", F::Simple(S::I32)),
    ("ExperienceSerializerPacket", "Name", F::Simple(S::Str)),
    ("ExperienceSerializerPacket", "StrongholdTurnsGiven", F::Simple(S::F32)),
    ("FactionDatabaseString", "PackageSerialized", F::Simple(S::U32)),
    ("FactionDatabaseString", "StringIDSerialized", F::Simple(S::I32)),
    ("FactionDatabaseString", "StringTableID", F::Simple(S::I32)),
    ("FactionDatabaseString", "StringTableSerialized", F::Simple(S::I32)),
    ("GameState", "ActiveScalers", F::Simple(S::I32)),
    ("GameState", "AutosaveCycleNumber", F::Simple(S::I32)),
    ("GameState", "CheatsEnabled", F::Simple(S::Bool)),
    ("GameState", "CouldAccessStashOnLastMap", F::Simple(S::Bool)),
    ("GameState", "CurrentMap", F::Class("MapData")),
    ("GameState", "CurrentMapIsStronghold", F::Simple(S::Bool)),
    ("GameState", "CurrentNextMap", F::Class("MapData")),
    ("GameState", "Difficulty", F::Simple(S::I32)),
    ("GameState", "ExpertMode", F::Simple(S::Bool)),
    ("GameState", "HasEnteredPX1", F::Simple(S::Bool)),
    ("GameState", "HasEnteredPX2", F::Simple(S::Bool)),
    ("GameState", "HasNotifiedPX2Installation", F::Simple(S::Bool)),
    ("GameState", "IgnoreInGrimoire", F::Simple(S::Bool)),
    ("GameState", "IgnoreSpellLimits", F::Simple(S::Bool)),
    ("GameState", "IsDifficultyPotd", F::Simple(S::Bool)),
    ("GameState", "IsDifficultyStoryTime", F::Simple(S::Bool)),
    ("GameState", "RetroactiveSpellMasteryChecked", F::Simple(S::Bool)),
    ("GameState", "TrialOfIron", F::Simple(S::Bool)),
    ("GameState", "TrialOfIronReadme", F::Class("TextAsset")),
    ("GameState", "UiWorldMapTutorialFinished", F::Simple(S::Bool)),
    ("GenericSpell", "CostToLearn", F::Simple(S::I32)),
    ("GenericSpell", "IsFree", F::Simple(S::Bool)),
    ("GenericSpell", "NeedsGrimoire", F::Simple(S::Bool)),
    ("GenericSpell", "ProhibitFromGrimoire", F::Simple(S::Bool)),
    ("GenericSpell", "SpellClass", F::Simple(S::I32)),
    ("GenericSpell", "SpellLevel", F::Simple(S::I32)),
    ("GenericSpell", "StatusEffectGrantingSpell", F::Class("StatusEffect")),
    ("GenericTalent", "Abilities", F::Class("GenericAbility")),
    ("GenericTalent", "AbilityMods", F::Class("AbilityMod")),
    ("GenericTalent", "Category", F::Simple(S::I32)),
    ("GenericTalent", "Description", F::Class("DatabaseString")),
    ("GenericTalent", "DisplayName", F::Class("DatabaseString")),
    ("GenericTalent", "Icon", F::Class("Texture2D")),
    ("GenericTalent", "SkillBonuses", F::Class("SkillBonus")),
    ("GenericTalent", "Type", F::Simple(S::I32)),
    ("Grimoire", "PrimaryOwnerName", F::Simple(S::Str)),
    ("Grimoire", "SerializedSpellNames", F::Class("List")),
    ("Grimoire", "SerializedSpells", F::Class("SpellChapter")),
    ("Grimoire", "Spells", F::Class("SpellChapter")),
    ("Hashtable", "Capacity", F::Simple(S::I32)),
    ("HashMap", "Capacity", F::Simple(S::I32)),
    ("HitData", "m_hitResult", F::Simple(S::I32)),
    ("HitData", "m_target", F::Class("GameObject")),
    ("InteractablesDatabaseString", "IsStringTableMutable", F::Simple(S::Bool)),
    ("InteractablesDatabaseString", "PackageSerialized", F::Simple(S::U32)),
    ("InteractablesDatabaseString", "StringIDSerialized", F::Simple(S::I32)),
    ("InteractablesDatabaseString", "StringTableID", F::Simple(S::I32)),
    ("InteractablesDatabaseString", "StringTableSerialized", F::Simple(S::I32)),
    ("InventoryItem", "AreaLootSource", F::Simple(S::Bool)),
    ("InventoryItem", "BaseItem", F::Simple(S::Str)),
    ("InventoryItem", "Original", F::Simple(S::Bool)),
    ("InventoryItem", "StackSize", F::Simple(S::I32)),
    ("InventoryItem", "stackSize", F::Simple(S::I32)),
    ("InventoryItem", "uiSlot", F::Simple(S::I32)),
    ("Item", "DescriptionText", F::Class("DatabaseString")),
    ("Item", "DisplayName", F::Class("DatabaseString")),
    ("Item", "DroppedItemContainer", F::Class("Container")),
    ("Item", "FilterType", F::Simple(S::I32)),
    ("Item", "FullValueSell", F::Simple(S::Bool)),
    ("Item", "IconLargeTexture", F::Class("Texture2D")),
    ("Item", "IconTexture", F::Class("Texture2D")),
    ("Item", "InventoryEquipSound", F::Simple(S::I32)),
    ("Item", "InventorySoundType", F::Simple(S::I32)),
    ("Item", "IsIngredient", F::Simple(S::Bool)),
    ("Item", "IsPrefab", F::Simple(S::Bool)),
    ("Item", "IsQuestItem", F::Simple(S::Bool)),
    ("Item", "IsRedirectingIngredient", F::Simple(S::Bool)),
    ("Item", "Location", F::Simple(S::I32)),
    ("Item", "MaxStackSize", F::Simple(S::I32)),
    ("Item", "Name", F::Simple(S::Str)),
    ("Item", "NeverDropAsLoot", F::Simple(S::Bool)),
    ("Item", "Prefab", F::Class("Item")),
    ("Item", "Renders", F::Simple(S::Bool)),
    ("Item", "StoredInventory", F::Class("BaseInventory")),
    ("Item", "Unique", F::Simple(S::Bool)),
    ("Item", "Value", F::Class("CurrencyValue")),
    ("KeyControl", "AltKey", F::Simple(S::Bool)),
    ("KeyControl", "CtrlKey", F::Simple(S::Bool)),
    ("KeyControl", "KeyCode", F::Simple(S::I32)),
    ("KeyControl", "ShiftKey", F::Simple(S::Bool)),
    ("LevelScaling", "BaseLevel", F::Simple(S::I32)),
    ("LevelScaling", "Empty", F::Simple(S::Bool)),
    ("LevelScaling", "LevelIncrement", F::Simple(S::I32)),
    ("LevelScaling", "MaxLevel", F::Simple(S::I32)),
    ("List", "Capacity", F::Simple(S::I32)),
    ("NoteEntry", "SerializedText", F::Simple(S::Str)),
    ("NoteEntry", "Text", F::Simple(S::Str)),
    ("NoteEntry", "Timestamp", F::Class("EternityDateTime")),
    ("NotesPage", "DisplayTitle", F::Simple(S::Str)),
    ("NotesPage", "LocalizedTitle", F::Class("DatabaseString")),
    ("NotesPage", "Notes", F::Class("List")),
    ("NotesPage", "SerializedNotes", F::Class("List")),
    ("NotesPage", "SerializedTitle", F::Simple(S::Str)),
    ("NotesPage", "UserTitle", F::Simple(S::Str)),
    ("OCL", "AdditionalInteractionObjects", F::Class("GameObject")),
    ("OCL", "Animation", F::Simple(S::I32)),
    ("OCL", "ArrivalDistance", F::Simple(S::F32)),
    ("OCL", "ArrivalRadius", F::Simple(S::F32)),
    ("OCL", "CurrentState", F::Simple(S::I32)),
    ("OCL", "HasInteractionObject", F::Simple(S::Bool)),
    ("OCL", "InteractionObject", F::Class("GameObject")),
    ("OCL", "IsRevealed", F::Simple(S::Bool)),
    ("OCL", "IsUsable", F::Simple(S::Bool)),
    ("OCL", "IsVisible", F::Simple(S::Bool)),
    ("OCL", "Key", F::Class("Item")),
    ("OCL", "LockDifficulty", F::Simple(S::I32)),
    ("OCL", "LockPickItem", F::Class("Item")),
    ("OCL", "MustHaveKey", F::Simple(S::Bool)),
    ("OCL", "RemoveUsedKey", F::Simple(S::Bool)),
    ("OCL", "StartingState", F::Simple(S::I32)),
    ("OCL", "UsableRadius", F::Simple(S::F32)),
    ("OCL", "UseRadius", F::Simple(S::F32)),
    ("ObjectPersistencePacket", "ComponentPackets", F::Class("ComponentPersistencePacket")),
    ("ObjectPersistencePacket", "GUID", F::Simple(S::Guid)),
    ("ObjectPersistencePacket", "Global", F::Simple(S::Bool)),
    ("ObjectPersistencePacket", "LevelName", F::Simple(S::Str)),
    ("ObjectPersistencePacket", "LoadManually", F::Simple(S::Bool)),
    ("ObjectPersistencePacket", "Location", F::Class("Vector3")),
    ("ObjectPersistencePacket", "Mobile", F::Simple(S::Bool)),
    ("ObjectPersistencePacket", "ObjectID", F::Simple(S::Str)),
    ("ObjectPersistencePacket", "ObjectName", F::Simple(S::Str)),
    ("ObjectPersistencePacket", "Packed", F::Simple(S::Bool)),
    ("ObjectPersistencePacket", "Parent", F::Simple(S::Str)),
    ("ObjectPersistencePacket", "PrefabResource", F::Simple(S::Str)),
    ("ObjectPersistencePacket", "Rotation", F::Class("Vector3")),
    ("Phrase", "Affliction", F::Class("Affliction")),
    ("Phrase", "BaseLinger", F::Simple(S::F32)),
    ("Phrase", "BaseRecitation", F::Simple(S::F32)),
    ("Phrase", "ChanterVisualEffectAttach", F::Simple(S::I32)),
    ("Phrase", "DefendedBy", F::Simple(S::I32)),
    ("Phrase", "Description", F::Class("DatabaseString")),
    ("Phrase", "DisplayName", F::Class("DatabaseString")),
    ("Phrase", "Duration", F::Simple(S::F32)),
    ("Phrase", "Icon", F::Class("Texture2D")),
    ("Phrase", "Level", F::Simple(S::I32)),
    ("Phrase", "Linger", F::Simple(S::F32)),
    ("Phrase", "OnChanterVisualEffect", F::Class("GameObject")),
    ("Phrase", "PhraseOwner", F::Class("CharacterStats")),
    ("Phrase", "Recitation", F::Simple(S::F32)),
    ("Phrase", "StatusEffects", F::Class("StatusEffectParams")),
    ("Phrase", "Tag", F::Simple(S::Str)),
    ("Phrase", "VocalizationNumber", F::Simple(S::I32)),
    ("Phrase", "m_phraseData", F::Class("PhraseData")),
    ("PhraseData", "AffectsStat", F::Simple(S::I32)),
    ("PhraseData", "AfflictionPrefab", F::Class("Affliction")),
    ("PhraseData", "Apply", F::Simple(S::I32)),
    ("PhraseData", "DmgType", F::Simple(S::I32)),
    ("PhraseData", "ExtraValue", F::Simple(S::F32)),
    ("PhraseData", "Icon", F::Class("Texture2D")),
    ("PhraseData", "IntervalRate", F::Simple(S::I32)),
    ("PhraseData", "IsHostile", F::Simple(S::Bool)),
    ("PhraseData", "OnAppliedVisualEffect", F::Class("GameObject")),
    ("PhraseData", "OnStartVisualEffect", F::Class("GameObject")),
    ("PhraseData", "OnStopVisualEffect", F::Class("GameObject")),
    ("PhraseData", "TrapPrefab", F::Class("Trap")),
    ("PhraseData", "Value", F::Simple(S::F32)),
    ("PhraseData", "VisualEffectAttach", F::Simple(S::I32)),
    ("PrerequisiteData", "AfflictionPrefab", F::Class("Affliction")),
    ("PrerequisiteData", "ClassValue", F::Simple(S::I32)),
    ("PrerequisiteData", "HideIfFailed", F::Simple(S::Bool)),
    ("PrerequisiteData", "IsConsumed", F::Simple(S::Bool)),
    ("PrerequisiteData", "RaceValue", F::Simple(S::I32)),
    ("PrerequisiteData", "SkillValue", F::Simple(S::I32)),
    ("PrerequisiteData", "Tag", F::Simple(S::Str)),
    ("PrerequisiteData", "Type", F::Simple(S::I32)),
    ("PrerequisiteData", "Value", F::Simple(S::F32)),
    ("QuestTimestamps", "addendumTimestamps", F::Class("EternityDateTime")),
    ("QuestTimestamps", "objectiveTimestamps", F::Class("EternityDateTime")),
    ("QuestTimestamps", "startTime", F::Class("EternityDateTime")),
    ("ReplaceObjectParams", "Existing", F::Class("GameObject")),
    ("ReplaceObjectParams", "ReplaceWith", F::Class("GameObject")),
    ("Reputation", "EditorName", F::Simple(S::Str)),
    ("Reputation", "FactionID", F::Simple(S::I32)),
    ("Reputation", "Name", F::Class("FactionDatabaseString")),
    ("Reputation", "Negative", F::Simple(S::I32)),
    ("Reputation", "Positive", F::Simple(S::I32)),
    ("Reputation", "Scale", F::Simple(S::I32)),
    ("Reputation", "SerializedName", F::Class("FactionDatabaseString")),
    ("Reputation", "TitleStrings", F::Class("TitleStringSet")),
    ("Reward", "MaxValue", F::Simple(S::I32)),
    ("Reward", "MinValue", F::Simple(S::I32)),
    ("Reward", "RewardType", F::Simple(S::I32)),
    ("Reward", "SpecificItem", F::Simple(S::Str)),
    ("Reward", "SpecificItemName", F::Simple(S::Str)),
    ("Reward", "m_maxValue", F::Simple(S::I32)),
    ("Reward", "m_minValue", F::Simple(S::I32)),
    ("Reward", "m_type", F::Simple(S::I32)),
    ("SkillBonus", "Bonus", F::Simple(S::I32)),
    ("SkillBonus", "Skill", F::Simple(S::I32)),
    ("SortedList", "Capacity", F::Simple(S::I32)),
    ("SpellChapter", "SerializedData", F::Class("List")),
    ("SpellChapter", "SpellData", F::Class("GenericSpell")),
    ("StatusEffect", "AbilitiesGrantedToTarget", F::Class("List")),
    ("StatusEffect", "AbilityOriginSerialized", F::Simple(S::Guid)),
    ("StatusEffect", "AbilityType", F::Simple(S::I32)),
    ("StatusEffect", "AfflictionKeyword", F::Simple(S::Str)),
    ("StatusEffect", "AfflictionOrigin", F::Simple(S::Str)),
    ("StatusEffect", "AfflictionOriginSerialized", F::Simple(S::Guid)),
    ("StatusEffect", "Applied", F::Simple(S::Bool)),
    ("StatusEffect", "AppliedFXNames", F::Class("String")),
    ("StatusEffect", "AppliedTriggered", F::Simple(S::Bool)),
    ("StatusEffect", "BundleId", F::Simple(S::I32)),
    ("StatusEffect", "BundleName", F::Simple(S::Str)),
    ("StatusEffect", "CachedTeam", F::Class("Team")),
    ("StatusEffect", "CurrentAppliedValue", F::Simple(S::F32)),
    ("StatusEffect", "CurrentAppliedValueForUi", F::Simple(S::F32)),
    ("StatusEffect", "DeleteOnClear", F::Simple(S::Bool)),
    ("StatusEffect", "Duration", F::Simple(S::F32)),
    ("StatusEffect", "DurationAfterBreak", F::Simple(S::F32)),
    ("StatusEffect", "EffectID", F::Simple(S::U32)),
    ("StatusEffect", "EquipmentOrigin", F::Simple(S::Str)),
    ("StatusEffect", "EquipmentOriginSerialized", F::Simple(S::Guid)),
    ("StatusEffect", "Exclusive", F::Simple(S::Bool)),
    ("StatusEffect", "Expired", F::Simple(S::Bool)),
    ("StatusEffect", "ExtraObjectSerialized", F::Simple(S::Guid)),
    ("StatusEffect", "ForcedAuraTargets", F::Class("List")),
    ("StatusEffect", "FriendlyRadius", F::Simple(S::F32)),
    ("StatusEffect", "HasBeenApplied", F::Simple(S::Bool)),
    ("StatusEffect", "HasTriggerActivation", F::Simple(S::Bool)),
    ("StatusEffect", "HitStrength", F::Simple(S::Str)),
    ("StatusEffect", "IgnoresValue", F::Simple(S::Bool)),
    ("StatusEffect", "IncreasePerTick", F::Simple(S::F32)),
    ("StatusEffect", "Interval", F::Simple(S::F32)),
    ("StatusEffect", "IsAura", F::Simple(S::Bool)),
    ("StatusEffect", "IsDOT", F::Simple(S::Bool)),
    ("StatusEffect", "IsDamageDealing", F::Simple(S::Bool)),
    ("StatusEffect", "IsDiseaseEffect", F::Simple(S::Bool)),
    ("StatusEffect", "IsFromAura", F::Simple(S::Bool)),
    ("StatusEffect", "IsNegativeMovementEffect", F::Simple(S::Bool)),
    ("StatusEffect", "IsOverTime", F::Simple(S::Bool)),
    ("StatusEffect", "IsPoisonEffect", F::Simple(S::Bool)),
    ("StatusEffect", "IsRecoveryTimePausingEffect", F::Simple(S::Bool)),
    ("StatusEffect", "IsScaledMultiplier", F::Simple(S::Bool)),
    ("StatusEffect", "IsSuppressed", F::Simple(S::Bool)),
    ("StatusEffect", "IsSuspended", F::Simple(S::Bool)),
    ("StatusEffect", "IsTriggerActivated", F::Simple(S::Bool)),
    ("StatusEffect", "LastsUntilCombatEnds", F::Simple(S::Bool)),
    ("StatusEffect", "LastsUntilRest", F::Simple(S::Bool)),
    ("StatusEffect", "NonstackingEffectType", F::Simple(S::I32)),
    ("StatusEffect", "Origin", F::Class("MonoBehaviour")),
    ("StatusEffect", "OwnerSerialized", F::Simple(S::Guid)),
    ("StatusEffect", "ParamPrereqSerialized", F::Class("List")),
    ("StatusEffect", "Params", F::Class("StatusEffectParams")),
    ("StatusEffect", "PhraseOrigin", F::Class("Phrase")),
    ("StatusEffect", "PhraseOriginSerialized", F::Simple(S::Guid)),
    ("StatusEffect", "RemovingEffect", F::Simple(S::Bool)),
    ("StatusEffect", "Scale", F::Simple(S::F32)),
    ("StatusEffect", "Slot", F::Simple(S::I32)),
    ("StatusEffect", "Source", F::Class("GameObject")),
    ("StatusEffect", "SourceSerialized", F::Simple(S::Guid)),
    ("StatusEffect", "Spells", F::Class("List")),
    ("StatusEffect", "Stackable", F::Simple(S::Bool)),
    ("StatusEffect", "Target", F::Class("GameObject")),
    ("StatusEffect", "TargetSerialized", F::Simple(S::Guid)),
    ("StatusEffect", "TemporaryDurationAdjustment", F::Simple(S::F32)),
    ("StatusEffect", "TicksAfterBreak", F::Simple(S::Bool)),
    ("StatusEffect", "TimeActive", F::Simple(S::F32)),
    ("StatusEffect", "TimeApplied", F::Simple(S::F32)),
    ("StatusEffect", "TimeLeft", F::Simple(S::F32)),
    ("StatusEffect", "Traps", F::Class("List")),
    ("StatusEffect", "TriggerDurationAdjustment", F::Simple(S::F32)),
    ("StatusEffect", "TriggerMaxTriggerCount", F::Simple(S::I32)),
    ("StatusEffect", "TriggerRadiusAdjustment", F::Simple(S::F32)),
    ("StatusEffect", "TriggerRemoveEffectAtMax", F::Simple(S::Bool)),
    ("StatusEffect", "TriggerResetTriggerOnEffectEnd", F::Simple(S::Bool)),
    ("StatusEffect", "TriggerResetTriggerOnEffectPulse", F::Simple(S::Bool)),
    ("StatusEffect", "TriggerType", F::Simple(S::I32)),
    ("StatusEffect", "TriggerValue", F::Simple(S::F32)),
    ("StatusEffect", "TriggerValueAdjustment", F::Simple(S::F32)),
    ("StatusEffect", "UiAuraEffectsApplied", F::Class("Map")),
    ("StatusEffect", "UnadjustedDurationAdd", F::Simple(S::F32)),
    ("StatusEffect", "m_applied", F::Simple(S::Bool)),
    ("StatusEffect", "m_damageToAbsorb", F::Simple(S::F32)),
    ("StatusEffect", "m_damageToReapply", F::Simple(S::F32)),
    ("StatusEffect", "m_deleteOnClear", F::Simple(S::Bool)),
    ("StatusEffect", "m_duration", F::Simple(S::F32)),
    ("StatusEffect", "m_durationOverride", F::Simple(S::F32)),
    ("StatusEffect", "m_durationScale", F::Simple(S::F32)),
    ("StatusEffect", "m_effect_is_on_main_target", F::Simple(S::Bool)),
    ("StatusEffect", "m_forceStackable", F::Simple(S::Bool)),
    ("StatusEffect", "m_generalCounter", F::Simple(S::U32)),
    ("StatusEffect", "m_id", F::Simple(S::U32)),
    ("StatusEffect", "m_intervalCount", F::Simple(S::U32)),
    ("StatusEffect", "m_intervalTimer", F::Simple(S::F32)),
    ("StatusEffect", "m_is_checking_melee_path", F::Simple(S::Bool)),
    ("StatusEffect", "m_needsDurationCalculated", F::Simple(S::Bool)),
    ("StatusEffect", "m_numRestCycles", F::Simple(S::I32)),
    ("StatusEffect", "m_religiousScale", F::Simple(S::F32)),
    ("StatusEffect", "m_scale", F::Simple(S::F32)),
    ("StatusEffect", "m_slot", F::Simple(S::I32)),
    ("StatusEffect", "m_stackingKey", F::Simple(S::I32)),
    ("StatusEffect", "m_suppressed", F::Simple(S::Bool)),
    ("StatusEffect", "m_suspensionCount", F::Simple(S::U32)),
    ("StatusEffect", "m_timeActive", F::Simple(S::F32)),
    ("StatusEffect", "m_timeApplied", F::Simple(S::F32)),
    ("StatusEffect", "m_triggerCount", F::Simple(S::I32)),
    ("StatusEffectLevelScaling", "BaseLevel", F::Simple(S::I32)),
    ("StatusEffectLevelScaling", "DurationAdjustment", F::Simple(S::F32)),
    ("StatusEffectLevelScaling", "Empty", F::Simple(S::Bool)),
    ("StatusEffectLevelScaling", "ExtraValueAdjustment", F::Simple(S::F32)),
    ("StatusEffectLevelScaling", "LevelIncrement", F::Simple(S::I32)),
    ("StatusEffectLevelScaling", "MaxLevel", F::Simple(S::I32)),
    ("StatusEffectLevelScaling", "ValueAdjustment", F::Simple(S::F32)),
    ("StatusEffectParams", "AffectsStat", F::Simple(S::I32)),
    ("StatusEffectParams", "AfflictionPrefabSerialized", F::Simple(S::Str)),
    ("StatusEffectParams", "ApplicationPrerequisites", F::Class("PrerequisiteData")),
    ("StatusEffectParams", "Apply", F::Simple(S::I32)),
    ("StatusEffectParams", "AttackPrefab", F::Class("AttackBase")),
    ("StatusEffectParams", "AttackPrefabSerialized", F::Simple(S::Guid)),
    ("StatusEffectParams", "AttackPrefabSerialized2", F::Class("AttackBase")),
    ("StatusEffectParams", "AttributeType", F::Simple(S::I32)),
    ("StatusEffectParams", "ChecksReligion", F::Simple(S::Bool)),
    ("StatusEffectParams", "ClassType", F::Simple(S::I32)),
    ("StatusEffectParams", "DefenseType", F::Simple(S::I32)),
    ("StatusEffectParams", "Description", F::Simple(S::Str)),
    ("StatusEffectParams", "DestroyVFXDelay", F::Simple(S::F32)),
    ("StatusEffectParams", "DmgType", F::Simple(S::I32)),
    ("StatusEffectParams", "DoNotScalePlayer", F::Simple(S::Bool)),
    ("StatusEffectParams", "DontHideFromLog", F::Simple(S::Bool)),
    ("StatusEffectParams", "Duration", F::Simple(S::F32)),
    ("StatusEffectParams", "EquippablePrefab", F::Class("Equippable")),
    ("StatusEffectParams", "EquippablePrefabSerialized", F::Simple(S::Guid)),
    ("StatusEffectParams", "EquippableSerialized2", F::Simple(S::Str)),
    ("StatusEffectParams", "ExtraValue", F::Simple(S::F32)),
    ("StatusEffectParams", "HideFromUi", F::Simple(S::Bool)),
    ("StatusEffectParams", "IgnoreAbilityDeactivation", F::Simple(S::Bool)),
    ("StatusEffectParams", "IntervalRate", F::Simple(S::I32)),
    ("StatusEffectParams", "IsCleanedUp", F::Simple(S::Bool)),
    ("StatusEffectParams", "IsHostile", F::Simple(S::Bool)),
    ("StatusEffectParams", "IsInstantApplication", F::Simple(S::Bool)),
    ("StatusEffectParams", "IsOverTime", F::Simple(S::Bool)),
    ("StatusEffectParams", "KeepOnDeath", F::Simple(S::Bool)),
    ("StatusEffectParams", "Keyword", F::Simple(S::Str)),
    ("StatusEffectParams", "LastsUntilCombatEnds", F::Simple(S::Bool)),
    ("StatusEffectParams", "LastsUntilRest", F::Simple(S::Bool)),
    ("StatusEffectParams", "LevelScaling", F::Class("StatusEffectLevelScaling")),
    ("StatusEffectParams", "LevelScalingBaseLevel", F::Simple(S::I32)),
    ("StatusEffectParams", "LevelScalingDurationAdjustment", F::Simple(S::F32)),
    ("StatusEffectParams", "LevelScalingExtraValueAdjustment", F::Simple(S::F32)),
    ("StatusEffectParams", "LevelScalingLevelIncrement", F::Simple(S::I32)),
    ("StatusEffectParams", "LevelScalingMaxLevel", F::Simple(S::I32)),
    ("StatusEffectParams", "LevelScalingValueAdjustment", F::Simple(S::F32)),
    ("StatusEffectParams", "MaxRestCycles", F::Simple(S::I32)),
    ("StatusEffectParams", "MergedValue", F::Simple(S::F32)),
    ("StatusEffectParams", "OnAppliedVisualEffect", F::Class("GameObject")),
    ("StatusEffectParams", "OnAuraVisualEffect", F::Class("GameObject")),
    ("StatusEffectParams", "OnDamageCallbackAbilitySerialized", F::Simple(S::Guid)),
    ("StatusEffectParams", "OnDamagedCallbackAbility", F::Class("GenericAbility")),
    ("StatusEffectParams", "OnStartVisualEffect", F::Class("GameObject")),
    ("StatusEffectParams", "OnStopVisualEffect", F::Class("GameObject")),
    ("StatusEffectParams", "OnTriggerVisualEffect", F::Class("GameObject")),
    ("StatusEffectParams", "OneHitUse", F::Simple(S::Bool)),
    ("StatusEffectParams", "Persistent", F::Simple(S::Bool)),
    ("StatusEffectParams", "RaceType", F::Simple(S::I32)),
    ("StatusEffectParams", "Tag", F::Simple(S::Str)),
    ("StatusEffectParams", "TemporaryDurationAdjustment", F::Simple(S::F32)),
    ("StatusEffectParams", "TrapPrefab", F::Class("Trap")),
    ("StatusEffectParams", "TrapPrefabSerialized", F::Simple(S::Guid)),
    ("StatusEffectParams", "TrapSerialized2", F::Class("Trap")),
    ("StatusEffectParams", "TriggerAdjustment", F::Class("StatusEffectTrigger")),
    ("StatusEffectParams", "UnadjustedDurationAdd", F::Simple(S::F32)),
    ("StatusEffectParams", "Value", F::Simple(S::F32)),
    ("StatusEffectParams", "VisualEffectAttach", F::Simple(S::I32)),
    ("StatusEffectParams", "m_deserializeInitialized", F::Simple(S::Bool)),
    ("StrongholdAdventure", "AdventureType", F::Simple(S::Str)),
    ("StrongholdAdventure", "DeferredXP", F::Simple(S::I32)),
    ("StrongholdAdventure", "Duration", F::Simple(S::I32)),
    ("StrongholdAdventure", "NumRewards", F::Simple(S::I32)),
    ("StrongholdAdventure", "PremadeAdventureIndex", F::Simple(S::I32)),
    ("StrongholdAdventure", "RewardList", F::Class("Reward")),
    ("StrongholdAdventure", "SerializedAdventureType", F::Simple(S::Str)),
    ("StrongholdAdventure", "SerializedAdventurer", F::Simple(S::Guid)),
    ("StrongholdAdventure", "SerializedDuration", F::Simple(S::I32)),
    ("StrongholdAdventure", "SerializedNumRewards", F::Simple(S::I32)),
    ("StrongholdAdventure", "SerializedOfferExpires", F::Simple(S::F32)),
    ("StrongholdAdventure", "SerializedRewardList", F::Class("Reward")),
    ("StrongholdAdventureCompletion", "AdventurerGuid", F::Simple(S::Guid)),
    ("StrongholdAdventureCompletion", "AdventurerName", F::Simple(S::Str)),
    ("StrongholdAdventureCompletion", "PremadeAdventureIndex", F::Simple(S::I32)),
    ("StrongholdAdventureCompletion", "RewardStrings", F::Class("List")),
    ("StrongholdAdventureCompletion", "Type", F::Simple(S::I32)),
    ("StrongholdDatabaseString", "PackageSerialized", F::Simple(S::U32)),
    ("StrongholdDatabaseString", "StringIDSerialized", F::Simple(S::I32)),
    ("StrongholdDatabaseString", "StringTableID", F::Simple(S::I32)),
    ("StrongholdDatabaseString", "StringTableSerialized", F::Simple(S::I32)),
    ("StrongholdEvent", "EventAttackNameSerialized", F::Simple(S::Str)),
    ("StrongholdEvent", "EventDataInt", F::Simple(S::I32)),
    ("StrongholdEvent", "EventType", F::Simple(S::Str)),
    ("StrongholdEvent", "EventUpgradeTypeSerialized", F::Simple(S::I32)),
    ("StrongholdEvent", "EventVisitorNameSerialized", F::Class("StrongholdVisitorSerializeData")),
    ("StrongholdEvent", "RandomValue1", F::Simple(S::I32)),
    ("StrongholdEvent", "RandomValue2", F::Simple(S::I32)),
    ("StrongholdEvent", "SerializedAbandonedCompanion", F::Simple(S::Guid)),
    ("StrongholdEvent", "SerializedCompanion", F::Simple(S::Guid)),
    ("StrongholdEvent", "Time", F::Simple(S::F32)),
    ("StrongholdGuestHireling", "CanHireGlobalVariableName", F::Simple(S::Str)),
    ("StrongholdGuestHireling", "CostPerDay", F::Simple(S::I32)),
    ("StrongholdGuestHireling", "HiredGlobalVariableName", F::Simple(S::Str)),
    ("StrongholdGuestHireling", "HirelingPrefab", F::Class("CharacterStats")),
    ("StrongholdGuestHireling", "IsLeaving", F::Simple(S::Bool)),
    ("StrongholdGuestHireling", "LeavesAfterFullPayCycle", F::Simple(S::Bool)),
    ("StrongholdGuestHireling", "MinimumPrestige", F::Simple(S::I32)),
    ("StrongholdGuestHireling", "Name", F::Simple(S::Str)),
    ("StrongholdGuestHireling", "Paid", F::Simple(S::Bool)),
    ("StrongholdGuestHireling", "PrestigeAdjustment", F::Simple(S::I32)),
    ("StrongholdGuestHireling", "SecurityAdjustment", F::Simple(S::I32)),
    ("StrongholdGuestHireling", "SerializedName", F::Simple(S::Str)),
    ("StrongholdGuestHireling", "SerializedNameId", F::Simple(S::I32)),
    ("StrongholdHireling", "CanHireGlobalVariableName", F::Simple(S::Str)),
    ("StrongholdHireling", "CostPerDay", F::Simple(S::I32)),
    ("StrongholdHireling", "HiredGlobalVariableName", F::Simple(S::Str)),
    ("StrongholdHireling", "HirelingPrefab", F::Class("CharacterStats")),
    ("StrongholdHireling", "IsLeaving", F::Simple(S::Bool)),
    ("StrongholdHireling", "LeavesAfterFullPayCycle", F::Simple(S::Bool)),
    ("StrongholdHireling", "Name", F::Simple(S::Str)),
    ("StrongholdHireling", "Paid", F::Simple(S::Bool)),
    ("StrongholdHireling", "PrestigeAdjustment", F::Simple(S::I32)),
    ("StrongholdHireling", "SecurityAdjustment", F::Simple(S::I32)),
    ("StrongholdHireling", "SerializedName", F::Simple(S::Str)),
    ("StrongholdHireling", "SerializedNameId", F::Simple(S::I32)),
    ("StrongholdPrisonerData", "GlobalVariableName", F::Simple(S::Str)),
    ("StrongholdPrisonerData", "PrisonerDescription", F::Class("StrongholdDatabaseString")),
    ("StrongholdPrisonerData", "PrisonerName", F::Class("CharacterDatabaseString")),
    ("StrongholdVisitorArrival", "Tag", F::Simple(S::Str)),
    ("StrongholdVisitorArrival", "TimeToArrive", F::Simple(S::F32)),
    ("StrongholdVisitorSerializeData", "AssociatedPrisoner", F::Class("CharacterDatabaseString")),
    ("StrongholdVisitorSerializeData", "Tag", F::Simple(S::Str)),
    ("StrongholdVisitorSerializeData", "TimeToLeave", F::Simple(S::F32)),
    ("Team", "DefaultRelationship", F::Simple(S::I32)),
    ("Team", "FriendlyTeams", F::Class("List")),
    ("Team", "GameFaction", F::Simple(S::I32)),
    ("Team", "HostileTeams", F::Class("List")),
    ("Team", "InjuredReputationChange", F::Simple(S::I32)),
    ("Team", "MurderedReputationChange", F::Simple(S::I32)),
    ("Team", "NeutralTeams", F::Class("List")),
    ("Team", "RestoredTeam", F::Simple(S::Bool)),
    ("Team", "ScriptTag", F::Simple(S::Str)),
    ("Team", "Tag", F::Simple(S::Str)),
    ("Team", "hideFlags", F::Simple(S::I32)),
    ("Team", "m_scriptTag", F::Simple(S::Str)),
    ("Team", "name", F::Simple(S::Str)),
    ("TextAsset", "bytes", F::Simple(S::Bytes)),
    ("TextAsset", "text", F::Simple(S::Str)),
    ("Timer", "Dead", F::Simple(S::Bool)),
    ("Timer", "Delay", F::Simple(S::F32)),
    ("Timer", "State", F::Simple(S::Bool)),
    ("Timer", "Target", F::Class("GameObject")),
    ("Timer", "TargetGuid", F::Simple(S::Guid)),
    ("TitleStringSet", "SerializedBadDescription", F::Class("FactionDatabaseString")),
    ("TitleStringSet", "SerializedBadTitles", F::Class("FactionDatabaseString")),
    ("TitleStringSet", "SerializedDefaultDescription", F::Class("FactionDatabaseString")),
    ("TitleStringSet", "SerializedDefaultTitles", F::Class("FactionDatabaseString")),
    ("TitleStringSet", "SerializedGoodDescription", F::Class("FactionDatabaseString")),
    ("TitleStringSet", "SerializedGoodTitles", F::Class("FactionDatabaseString")),
    ("TitleStringSet", "SerializedMixedDescription", F::Class("FactionDatabaseString")),
    ("TitleStringSet", "SerializedMixedTitles", F::Class("FactionDatabaseString")),
    ("Trap", "AbilityOverride", F::Class("GenericAbility")),
    ("Trap", "AbilityOverrides", F::Class("GenericAbility")),
    ("Trap", "ActivatesForAnyone", F::Simple(S::Bool)),
    ("Trap", "AllowMultiple", F::Simple(S::Bool)),
    ("Trap", "ArrivalRadius", F::Simple(S::F32)),
    ("Trap", "ButNotForCaster", F::Simple(S::Bool)),
    ("Trap", "CanDisarm", F::Simple(S::Bool)),
    ("Trap", "DestroyWhenDisarmed", F::Simple(S::Bool)),
    ("Trap", "DisarmItem", F::Class("Item")),
    ("Trap", "DisarmRadius", F::Simple(S::F32)),
    ("Trap", "Disarmed", F::Simple(S::Bool)),
    ("Trap", "DisplayName", F::Class("InteractablesDatabaseString")),
    ("Trap", "FromItem", F::Simple(S::Bool)),
    ("Trap", "IsPlayerOwnedTrap", F::Simple(S::Bool)),
    ("Trap", "IsUsable", F::Simple(S::Bool)),
    ("Trap", "IsWallTrap", F::Simple(S::Bool)),
    ("Trap", "MaxHitCount", F::Simple(S::I32)),
    ("Trap", "OneRandomAbility", F::Simple(S::Bool)),
    ("Trap", "Owner", F::Class("GameObject")),
    ("Trap", "SelfDestructiveTime", F::Simple(S::F32)),
    ("Trap", "SourceOverride", F::Class("CharacterStats")),
    ("Trap", "SuppressTriggerBark", F::Simple(S::Bool)),
    ("Trap", "TargetOverride", F::Class("GameObject")),
    ("Trap", "TrapCanPulse", F::Simple(S::Bool)),
    ("Trap", "TrapDifficulty", F::Simple(S::I32)),
    ("Trap", "TrapID", F::Simple(S::I32)),
    ("Trap", "TriggerDelay", F::Simple(S::F32)),
    ("Trap", "TriggerRadius", F::Simple(S::F32)),
    ("Trap", "UsableRadius", F::Simple(S::F32)),
    ("Trap", "Visible", F::Simple(S::Bool)),
    ("UIEquippedItem", "item", F::Class("Item")),
    ("UIInventoryFilter", "ButtonSprite", F::Class("UISprite")),
    ("UIInventoryFilter", "FilterType", F::Simple(S::I32)),
    ("UIInventoryFilter", "Selected", F::Simple(S::Bool)),
    ("UIInventoryFilter", "SelectedSprite", F::Class("UISprite")),
    ("Usable", "AdditionalInteractionObjects", F::Class("GameObject")),
    ("Usable", "Animation", F::Simple(S::I32)),
    ("Usable", "ArrivalRadius", F::Simple(S::F32)),
    ("Usable", "HasInteractionObject", F::Simple(S::Bool)),
    ("Usable", "InteractionObject", F::Class("GameObject")),
    ("Usable", "IsRevealed", F::Simple(S::Bool)),
    ("Usable", "IsUsable", F::Simple(S::Bool)),
    ("Usable", "IsVisible", F::Simple(S::Bool)),
    ("Usable", "UsableRadius", F::Simple(S::F32)),
    ("Vector2", "x", F::Simple(S::F32)),
    ("Vector2", "y", F::Simple(S::F32)),
    ("Vector3", "x", F::Simple(S::F32)),
    ("Vector3", "y", F::Simple(S::F32)),
    ("Vector3", "z", F::Simple(S::F32)),
];

fn fields() -> &'static HashMap<(&'static str, &'static str), F> {
    static M: OnceLock<HashMap<(&'static str, &'static str), F>> = OnceLock::new();
    M.get_or_init(|| FIELDS.iter().map(|(c, f, t)| ((*c, *f), *t)).collect())
}

/// Difficulty names, in the game's order (`GameDifficulty`).
const DIFFICULTIES: &[&str] = &["Easy", "Normal", "Hard", "Path of the Damned", "Story Time"];
/// The same, as the game's own names (written in `saveinfo.xml`).
const DIFFICULTY_IDS: &[&str] = &["Easy", "Normal", "Hard", "PathOfTheDamned", "StoryTime"];
/// Player classes, in the game's order (`CharacterStats+Class`); later values are creatures.
const CLASSES: &[&str] =
    &["Undefined", "Fighter", "Rogue", "Priest", "Wizard", "Barbarian", "Ranger", "Druid", "Paladin", "Monk", "Cipher", "Chanter"];
/// Player races (`CharacterStats+Race`); the rest are creature types.
const RACES: &[&str] = &["Undefined", "Human", "Elf", "Dwarf", "Godlike", "Orlan", "Undead", "Aumaua"];

/// The class a type string names: `CharacterStats+Race, Assembly-CSharp` → `Race`,
/// `System.Collections.Generic.List`1[[…]], mscorlib` → `List`.
fn short(t: &str) -> &str {
    let end = t.find(['`', ',']).unwrap_or(t.len());
    let base = t[..end].trim();
    base.rsplit(['.', '+']).next().unwrap_or(base)
}

/// How a value of this type is stored, for the simple types (enums are 32-bit numbers).
fn simple_of(t: &str) -> Option<S> {
    let base = t.split(',').next().unwrap_or(t).trim();
    Some(match base {
        "System.Boolean" => S::Bool,
        "System.Byte" => S::U8,
        "System.SByte" => S::I8,
        "System.Int16" => S::I16,
        "System.UInt16" => S::U16,
        "System.Int32" => S::I32,
        "System.UInt32" => S::U32,
        "System.Int64" | "System.DateTime" | "System.TimeSpan" => S::I64,
        "System.UInt64" => S::U64,
        "System.Single" => S::F32,
        "System.Double" => S::F64,
        "System.Char" => S::Char,
        "System.Decimal" => S::Dec,
        "System.String" | "System.Type" => S::Str,
        "System.Guid" => S::Guid,
        "System.Byte[]" => S::Bytes,
        b if b.starts_with("System.") || b.contains('[') || b.is_empty() => return None,
        _ => S::I32,
    })
}

/// A value's type: as written in the stream, or as its field declares it.
#[derive(Clone, Debug)]
enum Ty {
    Named(Arc<str>),
    Declared(F),
    Unknown,
}

impl Ty {
    fn class(&self) -> Option<&str> {
        match self {
            Ty::Named(s) => Some(short(s)),
            Ty::Declared(F::Class(c)) => Some(c),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
struct Leaf {
    start: usize,
    end: usize,
    s: S,
    /// Names of an enum's values (the drop-down), when the screen offers one.
    names: Option<&'static [&'static str]>,
}

#[derive(Debug)]
enum Body {
    Null,
    Ref,
    /// The value and its leaf (None: read-only); None when the value is null.
    Simple(Option<(Value, Option<usize>)>),
    Props(Vec<Node>),
    Items(Vec<Node>),
    Dict(Vec<(Node, Node)>),
}

#[derive(Debug)]
struct Node {
    name: Option<Arc<str>>,
    body: Body,
}

impl Node {
    fn prop(&self, name: &str) -> Option<&Node> {
        match &self.body {
            Body::Props(p) => p.iter().find(|n| n.name.as_deref() == Some(name)),
            _ => None,
        }
    }

    fn text(&self) -> Option<&str> {
        match &self.body {
            Body::Simple(Some((Value::String(s), _))) => Some(s),
            _ => None,
        }
    }
}

struct R<'a> {
    d: &'a [u8],
    pos: usize,
    leaves: Vec<Leaf>,
    names: Vec<Option<Arc<str>>>,
    types: Vec<Option<Arc<str>>>,
    /// How deep the current value is nested (a crafted file could nest without end).
    depth: usize,
}

/// Deeper nesting than any save has is refused, so reading and showing it can't run out of stack.
const MAX_DEPTH: usize = 64;

type Res<T> = Result<T, String>;

impl R<'_> {
    fn take(&mut self, n: usize) -> Res<&[u8]> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.d.len()).ok_or_else(|| format!("the data ends early at {}", self.pos))?;
        let s = &self.d[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn u8(&mut self) -> Res<u8> {
        Ok(self.take(1)?[0])
    }

    /// A count or index: a size byte, then that many bytes.
    fn num(&mut self) -> Res<usize> {
        let n = match self.u8()? {
            0 => 0,
            1 => self.u8()? as i64,
            2 => i16::from_le_bytes(self.take(2)?.try_into().unwrap()) as i64,
            _ => i32::from_le_bytes(self.take(4)?.try_into().unwrap()) as i64,
        };
        usize::try_from(n).map_err(|_| format!("a negative number at {}", self.pos))
    }

    /// A count of things that follow: each takes at least a byte.
    fn count(&mut self) -> Res<usize> {
        let n = self.num()?;
        if n > self.d.len() - self.pos {
            return Err(format!("a count that doesn't fit at {}", self.pos));
        }
        Ok(n)
    }

    fn len7(&mut self) -> Res<usize> {
        let mut n = 0usize;
        for shift in (0..35).step_by(7) {
            let b = self.u8()?;
            n |= ((b & 0x7f) as usize) << shift;
            if b & 0x80 == 0 {
                return Ok(n);
            }
        }
        Err("a bad length".into())
    }

    fn text(&mut self) -> Res<String> {
        let n = self.len7()?;
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| "text that isn't UTF-8".to_string())
    }

    fn table(&mut self) -> Res<Vec<Option<Arc<str>>>> {
        let n = self.count()?;
        let mut out = Vec::with_capacity(n.min(4096));
        for _ in 0..n {
            out.push(if self.u8()? != 0 { Some(Arc::from(self.text()?)) } else { None });
        }
        Ok(out)
    }

    /// One stream: its name and type tables, then its root.
    fn stream(&mut self) -> Res<Node> {
        self.names = self.table()?;
        self.types = self.table()?;
        let eid = self.u8()?;
        self.node(eid, Ty::Unknown, None)
    }

    fn name(&mut self) -> Res<Option<Arc<str>>> {
        let i = self.num()?;
        self.names.get(i).cloned().ok_or_else(|| format!("name {i} is not in the table"))
    }

    fn ty(&mut self) -> Res<Option<Arc<str>>> {
        let i = self.num()?;
        self.types.get(i).cloned().ok_or_else(|| format!("type {i} is not in the table"))
    }

    fn node(&mut self, eid: u8, expected: Ty, owner: Option<&str>) -> Res<Node> {
        if self.depth >= MAX_DEPTH {
            return Err(format!("values nested too deep at {}", self.pos));
        }
        self.depth += 1;
        let n = self.node_in(eid, expected, owner);
        self.depth -= 1;
        n
    }

    fn node_in(&mut self, eid: u8, expected: Ty, owner: Option<&str>) -> Res<Node> {
        let name = self.name()?;
        let ty = match self.ty()? {
            Some(t) => Ty::Named(t),
            None => match expected {
                Ty::Unknown => match (owner, name.as_deref()) {
                    (Some(o), Some(n)) => fields().get(&(o, n)).map_or(Ty::Unknown, |f| Ty::Declared(*f)),
                    _ => Ty::Unknown,
                },
                e => e,
            },
        };
        // Elements: 1 collection, 2 object, 3 dictionary, 4 multi-dimensional array, 5 null,
        // 6 simple value, 7 array; 8 and 10–13 are the same with a reference id; 9 a reference.
        if matches!(eid, 8..=13) {
            self.num()?;
        }
        let body = match eid {
            9 => Body::Ref,
            5 => Body::Null,
            6 => self.simple(&ty)?,
            2 | 8 => Body::Props(self.props(ty.class())?),
            1 | 10 => {
                let et = self.ty()?;
                self.props(ty.class())?;
                Body::Items(self.items(et, ty.class())?)
            }
            3 | 11 => {
                let (kt, vt) = (self.ty()?, self.ty()?);
                self.props(ty.class())?;
                let n = self.count()?;
                let mut kv = Vec::with_capacity(n.min(4096));
                for _ in 0..n {
                    let e = self.u8()?;
                    let k = self.node(e, kt.clone().map_or(Ty::Unknown, Ty::Named), ty.class())?;
                    let e = self.u8()?;
                    let v = self.node(e, vt.clone().map_or(Ty::Unknown, Ty::Named), ty.class())?;
                    kv.push((k, v));
                }
                Body::Dict(kv)
            }
            7 | 12 => {
                let et = self.ty()?;
                self.num()?; // lower bound
                Body::Items(self.items(et, ty.class())?)
            }
            _ => return Err(format!("element {eid} at {} is not read", self.pos)),
        };
        Ok(Node { name, body })
    }

    fn props(&mut self, owner: Option<&str>) -> Res<Vec<Node>> {
            let owner: Option<String> = owner.map(String::from);
        let n = self.count()?;
        let mut out = Vec::with_capacity(n.min(4096));
        for _ in 0..n {
            let e = self.u8()?;
            out.push(self.node(e, Ty::Unknown, owner.as_deref())?);
        }
        Ok(out)
    }

    fn items(&mut self, et: Option<Arc<str>>, owner: Option<&str>) -> Res<Vec<Node>> {
            let owner: Option<String> = owner.map(String::from);
        let n = self.count()?;
        let mut out = Vec::with_capacity(n.min(4096));
        for _ in 0..n {
            let e = self.u8()?;
            out.push(self.node(e, et.clone().map_or(Ty::Unknown, Ty::Named), owner.as_deref())?);
        }
        Ok(out)
    }

    fn simple(&mut self, ty: &Ty) -> Res<Body> {
        if self.u8()? == 0 {
            return Ok(Body::Simple(None));
        }
        let (s, names) = match ty {
            Ty::Named(t) => (
                simple_of(t).ok_or_else(|| format!("type {t} at {} is not read", self.pos))?,
                match short(t) {
                    "GameDifficulty" => Some(DIFFICULTIES),
                    _ => None,
                },
            ),
            Ty::Declared(F::Simple(s)) => (*s, None),
            _ => return Err(format!("a value of unknown type at {}", self.pos)),
        };
        let start = self.pos;
        let le = |b: &[u8]| -> [u8; 8] {
            let mut a = [0u8; 8];
            a[..b.len()].copy_from_slice(b);
            a
        };
        let size = match s {
            S::Bool | S::U8 | S::I8 => 1,
            S::I16 | S::U16 | S::Char => 2,
            S::I32 | S::U32 | S::F32 => 4,
            S::I64 | S::U64 | S::F64 => 8,
            S::Dec | S::Guid => 16,
            S::Str | S::Bytes => 0,
        };
        let raw = self.take(size)?.to_vec();
        let mut editable = true;
        let v = match s {
            S::Bool => Value::Bool(raw[0] != 0),
            S::U8 => Value::from(raw[0]),
            S::I8 => Value::from(raw[0] as i8),
            S::I16 => Value::from(i16::from_le_bytes([raw[0], raw[1]])),
            S::U16 => Value::from(u16::from_le_bytes([raw[0], raw[1]])),
            S::Char => Value::String(char::from_u32(u16::from_le_bytes([raw[0], raw[1]]) as u32).unwrap_or('?').to_string()),
            S::I32 => {
                let n = i32::from_le_bytes(raw[..4].try_into().unwrap());
                match ty {
                    Ty::Named(t) => match (t.split(',').next().unwrap_or(""), names) {
                        (_, Some(list)) => match list.get(n as usize) {
                            Some(name) => Value::String(name.to_string()),
                            None => Value::from(n),
                        },
                        // Shown by name, read-only: changing a class or race takes more than one value.
                        (c @ ("CharacterStats+Class" | "CharacterStats+Race"), None) => {
                            let list = if c.ends_with("Class") { CLASSES } else { RACES };
                            editable = false;
                            list.get(n as usize).map_or(Value::from(n), |s| Value::String(s.to_string()))
                        }
                        _ => Value::from(n),
                    },
                    _ => Value::from(n),
                }
            }
            S::U32 => Value::from(u32::from_le_bytes(raw[..4].try_into().unwrap())),
            S::F32 => crate::tree::f32_value(f32::from_le_bytes(raw[..4].try_into().unwrap())),
            S::I64 => Value::from(i64::from_le_bytes(le(&raw))),
            S::U64 => Value::from(u64::from_le_bytes(le(&raw))),
            S::F64 => crate::tree::num(f64::from_le_bytes(le(&raw))),
            S::Dec => {
                editable = false;
                Value::String(raw.iter().map(|b| format!("{b:02x}")).collect())
            }
            S::Guid => {
                editable = false;
                let g = &raw;
                Value::String(format!(
                    "{:08x}-{:04x}-{:04x}-{}",
                    u32::from_le_bytes(g[0..4].try_into().unwrap()),
                    u16::from_le_bytes([g[4], g[5]]),
                    u16::from_le_bytes([g[6], g[7]]),
                    g[8..].iter().enumerate().map(|(i, b)| if i == 2 { format!("-{b:02x}") } else { format!("{b:02x}") }).collect::<String>()
                ))
            }
            S::Str => Value::String(self.text()?),
            S::Bytes => {
                editable = false;
                let n = self.num()?;
                self.take(n)?;
                Value::String(format!("<{n} bytes>"))
            }
        };
        let leaf = editable.then(|| {
            self.leaves.push(Leaf { start, end: self.pos, s, names: if matches!(s, S::I32) { names } else { None } });
            self.leaves.len() - 1
        });
        Ok(Body::Simple(Some((v, leaf))))
    }
}

/// Builds the display tree, noting the pointer of every editable value.
struct View {
    /// Sorted, so the values below a pointer are one range.
    editable: BTreeMap<String, usize>,
}

impl View {
    fn value(&mut self, n: &Node, ptr: &str) -> Value {
        match &n.body {
            Body::Null | Body::Simple(None) => Value::Null,
            Body::Ref => Value::String("<reference>".into()),
            Body::Simple(Some((v, leaf))) => {
                if let Some(l) = leaf {
                    self.editable.insert(ptr.to_string(), *l);
                }
                v.clone()
            }
            Body::Props(props) => {
                let mut m = Map::new();
                for p in props {
                    let key = unique_key(&m, p.name.as_deref().unwrap_or("value"));
                    let v = self.value(p, &child(ptr, &key));
                    m.insert(key, v);
                }
                Value::Object(m)
            }
            Body::Items(items) => Value::Array(items.iter().enumerate().map(|(i, it)| self.value(it, &format!("{ptr}/{i}"))).collect()),
            Body::Dict(kv) => {
                if kv.iter().all(|(k, _)| k.text().is_some()) {
                    let mut m = Map::new();
                    for (k, v) in kv {
                        let key = unique_key(&m, k.text().unwrap_or(""));
                        let val = self.value(v, &child(ptr, &key));
                        m.insert(key, val);
                    }
                    Value::Object(m)
                } else {
                    Value::Array(
                        kv.iter()
                            .enumerate()
                            .map(|(i, (k, v))| {
                                let p = format!("{ptr}/{i}");
                                let key = self.value(k, &format!("{p}/Key"));
                                let val = self.value(v, &format!("{p}/Value"));
                                serde_json::json!({ "Key": key, "Value": val })
                            })
                            .collect(),
                    )
                }
            }
        }
    }

    /// One saved object: its own fields, and its components by type with their variables.
    fn object(&mut self, n: &Node, ptr: &str) -> Value {
        let mut m = Map::new();
        let Body::Props(props) = &n.body else { return self.value(n, ptr) };
        for p in props {
            let name = match p.name.as_deref() {
                Some("ComponentPackets") => "Components",
                n => n.unwrap_or("value"),
            };
            let key = unique_key(&m, name);
            let at = child(ptr, &key);
            let v = match (&p.body, key.as_str()) {
                (Body::Items(comps), "Components") => {
                    let mut c = Map::new();
                    for comp in comps {
                        let ty = comp.prop("TypeString").and_then(Node::text).unwrap_or("Component");
                        let k = unique_key(&c, ty);
                        let v = match comp.prop("Variables") {
                            Some(vars) => self.value(vars, &child(&at, &k)),
                            None => Value::Null,
                        };
                        c.insert(k, v);
                    }
                    Value::Object(c)
                }
                _ => self.value(p, &at),
            };
            m.insert(key, v);
        }
        Value::Object(m)
    }
}

#[derive(Clone, Debug)]
struct Back {
    raw: Arc<Vec<u8>>,
    leaves: Arc<Vec<Leaf>>,
}

fn encode(l: &Leaf, v: &Value) -> Result<Vec<u8>, String> {
    macro_rules! int {
        ($t:ty) => {
            (int_in(v, <$t>::MIN as i128, <$t>::MAX as i128, "This number")? as $t).to_le_bytes().to_vec()
        };
    }
    if let (Some(names), Value::String(s)) = (l.names, v) {
        let i = names.iter().position(|n| n == s).ok_or_else(|| format!("{s} is not one of the choices"))?;
        return Ok((i as i32).to_le_bytes().to_vec());
    }
    Ok(match l.s {
        S::Bool => vec![match v {
            Value::Bool(b) => *b as u8,
            _ => int_in(v, 0, 1, "This value")? as u8,
        }],
        S::U8 => int!(u8),
        S::I8 => int!(i8),
        S::I16 => int!(i16),
        S::U16 => int!(u16),
        S::I32 => int!(i32),
        S::U32 => int!(u32),
        S::I64 => int!(i64),
        S::U64 => int!(u64),
        S::F32 => {
            let f = v.as_f64().ok_or("This value must be a number")?;
            if !f.is_finite() || f.abs() > f32::MAX as f64 {
                return Err("This number is too big".into());
            }
            (f as f32).to_le_bytes().to_vec()
        }
        S::F64 => v.as_f64().filter(|f| f.is_finite()).ok_or("This value must be a number")?.to_le_bytes().to_vec(),
        S::Char => {
            let s = v.as_str().ok_or("This value must be one letter")?;
            let mut it = s.encode_utf16();
            match (it.next(), it.next()) {
                (Some(c), None) => c.to_le_bytes().to_vec(),
                _ => return Err("This value must be one letter".into()),
            }
        }
        S::Str => {
            let s = v.as_str().ok_or("This value must be text")?;
            let mut out = Vec::new();
            let mut n = s.len();
            loop {
                let b = (n & 0x7f) as u8;
                n >>= 7;
                if n == 0 {
                    out.push(b);
                    break;
                }
                out.push(b | 0x80);
            }
            out.extend_from_slice(s.as_bytes());
            out
        }
        S::Dec | S::Guid | S::Bytes => return Err("this value can't be changed".into()),
    })
}

impl Backend for Back {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (id, v) in changes {
            let l = &self.leaves[*id];
            p.add(l.start, l.end, encode(l, v)?);
        }
        p.apply(&self.raw)
    }

    fn normalize(&self, leaf: usize, v: Value) -> Value {
        if self.leaves[leaf].s == S::F32 {
            crate::tree::round_f32(v)
        } else {
            v
        }
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        encode(&self.leaves[leaf], v).map(|_| ())
    }

    fn choices(&self, leaf: usize, _view: &Value) -> Option<Arc<Vec<String>>> {
        self.leaves[leaf].names.map(|n| Arc::new(n.iter().map(|s| s.to_string()).collect()))
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// The first stream: names `Root`, types `System.Int32`, then the object count.
pub fn looks(d: &[u8]) -> bool {
    if d.len() < 40 || !d.starts_with(&[1, 1, 1, 4, b'R', b'o', b'o', b't', 1, 1, 1]) {
        return false;
    }
    // The type's length (one byte, or two for a long assembly-qualified name), then its name.
    let at = if d[11] & 0x80 == 0 { 12 } else { 13 };
    d[at..].starts_with(b"System.Int32,")
}

/// Every object of the save, parsed.
fn read(d: &[u8]) -> Res<(Vec<Node>, Vec<Leaf>)> {
    if !looks(d) {
        return Err("not a Pillars of Eternity object list".into());
    }
    let mut r = R { d, pos: 0, leaves: vec![], names: vec![], types: vec![], depth: 0 };
    let root = r.stream()?;
    let count = match root.body {
        Body::Simple(Some((Value::Number(n), _))) => n.as_u64().ok_or("a bad object count")? as usize,
        _ => return Err("no object count".into()),
    };
    if count > d.len() {
        return Err("a bad object count".into());
    }
    r.leaves.clear();
    let mut objects = Vec::with_capacity(count.min(4096));
    for _ in 0..count {
        objects.push(r.stream()?);
    }
    if r.pos != d.len() {
        return Err(format!("{} bytes left after the last object", d.len() - r.pos));
    }
    Ok((objects, r.leaves))
}

/// The display name of an item from its prefab path: `…/PLANT_Admeths_Wyrt.prefab` → `Admeths Wyrt`.
fn item_name(path: &str) -> String {
    let stem = path.rsplit('/').next().unwrap_or(path);
    let stem = stem.strip_suffix(".prefab").unwrap_or(stem);
    // Category prefixes (`ITEM_`, `PART_`, `PET_`) come off.
    let mut stem = stem;
    while let Some((head, rest)) = stem.split_once('_') {
        if rest.is_empty() || head.len() < 2 || !head.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
            break;
        }
        stem = rest;
    }
    stem.replace('_', " ")
}

/// A party member's name: the name the player gave, or the companion's.
fn character_name(object: &str, stats: &Value) -> String {
    if let Some(n) = stats.get("OverrideName").and_then(Value::as_str).filter(|s| !s.is_empty()) {
        return n.to_string();
    }
    let rest = object.split_once('_').map_or(object, |(_, r)| r);
    let base = rest.split('(').next().unwrap_or(rest);
    match base {
        "GGP" => "Durance".into(),
        "GM" => "Grieving Mother".into(),
        b => b.replace('_', " "),
    }
}

/// Copies the value at `from` to `to` in `out`, with every editable pointer below it.
fn alias(objects: &Value, editable: &mut BTreeMap<String, usize>, from: &str, to: &str, keep: impl Fn(&str) -> bool) -> Option<Value> {
    let v = objects.pointer(from.strip_prefix("/Objects").unwrap_or(from))?.clone();
    let below: Vec<(String, usize)> = editable
        .range(from.to_string()..)
        .map_while(|(k, l)| Some((k.strip_prefix(from)?, l)))
        .filter_map(|(rest, l)| (rest.is_empty() || rest.starts_with('/')).then(|| (format!("{to}{rest}"), *l)))
        .filter(|(k, _)| keep(k))
        .collect();
    editable.extend(below);
    Some(v)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (objects, leaves) = read(d)?;
    let mut view = View { editable: BTreeMap::new() };
    let mut all = Map::new();
    let mut keys = vec![];
    // Many objects share a name: count them instead of trying "name (2)", "name (3)"… each time.
    let mut seen: HashMap<String, usize> = HashMap::new();
    for o in &objects {
        let name = o.prop("ObjectName").and_then(Node::text).unwrap_or("Object");
        let n = seen.entry(name.to_string()).or_insert(1);
        let mut key = name.to_string();
        while all.contains_key(&key) {
            *n += 1;
            key = format!("{name} ({n})");
        }
        let v = view.object(o, &child("/Objects", &key));
        all.insert(key.clone(), v);
        keys.push(key);
    }
    let all = Value::Object(all);
    let mut ed = view.editable;
    let mut root = Map::new();

    // The game's settings: difficulty (the screen's drop-down) and the rest for the Tree.
    if let Some(k) = keys.iter().find(|k| k.starts_with("Global(Clone)")) {
        let from = format!("{}/Components/GameState", child("/Objects", k));
        if let Some(v) = alias(&all, &mut ed, &from, "/Game", |p| p == "/Game/Difficulty") {
            root.insert("Game".into(), v);
        }
    }

    // The player: money, camping supplies and the inventories.
    let mut party = vec![];
    for key in &keys {
        let lower = key.to_lowercase();
        let base = child("/Objects", key);
        let Some(stats) = all.pointer(&format!("{}/Components/CharacterStats", child("", key))) else { continue };
        if !(lower.starts_with("player_") || lower.starts_with("companion_")) {
            continue;
        }
        let mut m = Map::new();
        m.insert("Name".into(), Value::String(character_name(key, stats)));
        let from = format!("{base}/Components/CharacterStats");
        let at = format!("/Party/{}", party.len());
        for (label, field) in [
            ("Level", "Level"),
            ("Experience", "Experience"),
            ("Class", "CharacterClass"),
            ("Race", "CharacterRace"),
            ("Might", "BaseMight"),
            ("Constitution", "BaseConstitution"),
            ("Dexterity", "BaseDexterity"),
            ("Perception", "BasePerception"),
            ("Intellect", "BaseIntellect"),
            ("Resolve", "BaseResolve"),
            ("Athletics", "AthleticsSkill"),
            ("Lore", "LoreSkill"),
            ("Mechanics", "MechanicsSkill"),
            ("Survival", "SurvivalSkill"),
            ("Stealth", "StealthSkill"),
            ("Skill points to spend", "RemainingSkillPoints"),
        ] {
            // The level follows experience in the game's level-up screen; it is shown only.
            let keep = |p: &str| label != "Level" && !p.is_empty();
            if let Some(v) = alias(&all, &mut ed, &child(&from, field), &child(&at, label), keep) {
                m.insert(label.into(), v);
            }
        }
        if let Some(v) = alias(&all, &mut ed, &format!("{base}/Components/Health/CurrentHealth"), &format!("{at}/Health"), |_| false) {
            m.insert("Health".into(), v);
        }
        party.push(Value::Object(m));
        // The first player object is the player (its money and inventories).
        if !lower.starts_with("player_") || root.contains_key("Player") {
            continue;
        }
        let mut player = Map::new();
        let inv = format!("{base}/Components/PlayerInventory");
        if let Some(v) = alias(&all, &mut ed, &format!("{inv}/currencyTotalValue/v"), "/Player/Money", |_| true) {
            player.insert("Money".into(), v);
        }
        if let Some(v) = alias(&all, &mut ed, &format!("{inv}/campingSupplies"), "/Player/Camping supplies", |_| true) {
            player.insert("Camping supplies".into(), v);
        }
        for (label, comp) in [("Inventory", "PlayerInventory"), ("Stash", "StashInventory"), ("Ingredients", "CraftingInventory")] {
            let from = format!("{base}/Components/{comp}/ItemList");
            let to = format!("/Player/{label}");
            let keep = |p: &str| p.ends_with("/StackSize") || p.ends_with("/stackSize");
            if let Some(Value::Array(items)) = alias(&all, &mut ed, &from, &to, keep) {
                let items = items
                    .into_iter()
                    .map(|mut it| {
                        if let Some(o) = it.as_object_mut() {
                            let name = o.get("BaseItem").and_then(Value::as_str).map(item_name).unwrap_or_default();
                            o.insert("Item".into(), Value::String(name));
                        }
                        it
                    })
                    .collect();
                player.insert(label.into(), Value::Array(items));
            }
        }
        root.insert("Player".into(), Value::Object(player));
    }
    if !party.is_empty() {
        root.insert("Party".into(), Value::Array(party));
    }
    root.insert("Objects".into(), all);
    let back = Back { raw: Arc::new(d.to_vec()), leaves: Arc::new(leaves) };
    let doc = TreeDoc::new(Value::Object(root), ed.into_iter().collect(), KIND, Box::new(back)).with_advanced(vec!["Objects".into()]);
    Ok(doc)
}

/// The game's difficulty in an object list, by its `saveinfo.xml` name.
pub fn difficulty_id(d: &[u8]) -> Option<&'static str> {
    let doc = parse(d).ok()?;
    let name = doc.view.pointer("/Game/Difficulty")?.as_str()?;
    DIFFICULTIES.iter().position(|n| *n == name).map(|i| DIFFICULTY_IDS[i])
}

/// `saveinfo.xml` with its difficulty set to `id` (the load screen shows it), or None when it
/// already says so or has no difficulty.
pub fn saveinfo_with_difficulty(xml: &[u8], id: &str) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(xml).ok()?;
    let tag = text.find("name=\"Difficulty\"")?;
    let close = text[tag..].find('>')? + tag;
    let v = text[tag..close].find("value=\"")? + tag + "value=\"".len();
    let end = text[v..].find('"')? + v;
    if &text[v..end] == id {
        return None;
    }
    Some([&xml[..v], id.as_bytes(), &xml[end..]].concat())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes streams the way the game does, for a small save made in the test.
    #[derive(Default)]
    struct W {
        names: Vec<&'static str>,
        types: Vec<Option<&'static str>>,
        body: Vec<u8>,
    }

    fn text(out: &mut Vec<u8>, s: &str) {
        out.push(s.len() as u8);
        out.extend_from_slice(s.as_bytes());
    }

    impl W {
        fn num(&mut self, n: usize) {
            self.body.extend_from_slice(&[1, n as u8]);
        }
        fn name(&mut self, n: &'static str) {
            let i = self.names.iter().position(|x| *x == n).unwrap_or_else(|| {
                self.names.push(n);
                self.names.len() - 1
            });
            self.num(i);
        }
        fn ty(&mut self, t: Option<&'static str>) {
            let i = self.types.iter().position(|x| *x == t).unwrap_or_else(|| {
                self.types.push(t);
                self.types.len() - 1
            });
            self.num(i);
        }
        fn head(&mut self, eid: u8, name: &'static str, ty: Option<&'static str>) {
            self.body.push(eid);
            self.name(name);
            self.ty(ty);
        }
        fn int(&mut self, name: &'static str, ty: Option<&'static str>, v: i32) {
            self.head(6, name, ty);
            self.body.push(1);
            self.body.extend_from_slice(&v.to_le_bytes());
        }
        fn string(&mut self, name: &'static str, ty: Option<&'static str>, v: &str) {
            self.head(6, name, ty);
            self.body.push(1);
            text(&mut self.body, v);
        }
        /// One `Variables` entry: a string key, then the value written by `f`.
        fn var(&mut self, key: &str, f: impl FnOnce(&mut Self)) {
            self.string("", Some(STR), key);
            f(self);
        }
        fn finish(self) -> Vec<u8> {
            let mut out = vec![1, self.names.len() as u8];
            for n in &self.names {
                out.push(1);
                text(&mut out, n);
            }
            out.extend_from_slice(&[1, self.types.len() as u8]);
            for t in &self.types {
                match t {
                    Some(t) => {
                        out.push(1);
                        text(&mut out, t);
                    }
                    None => out.push(0),
                }
            }
            out.extend(self.body);
            out
        }
    }

    const STR: &str = "System.String, mscorlib";
    const INT: &str = "System.Int32, mscorlib";
    const OBJ: &str = "System.Object, mscorlib";

    /// An object with one component: `vars` writes the variables (count first).
    fn object(name: &'static str, comps: &[(&'static str, usize, &dyn Fn(&mut W))]) -> Vec<u8> {
        let mut w = W::default();
        w.head(2, "Root", Some("ObjectPersistencePacket, Assembly-CSharp"));
        w.num(2);
        w.string("ObjectName", None, name);
        w.head(7, "ComponentPackets", Some("ComponentPersistencePacket[], Assembly-CSharp"));
        w.ty(Some("ComponentPersistencePacket, Assembly-CSharp"));
        w.num(0);
        w.num(comps.len());
        for (ty, n, vars) in comps {
            w.head(2, "", None);
            w.num(2);
            w.string("TypeString", None, ty);
            w.head(3, "Variables", Some("System.Collections.Generic.Dictionary`2[[System.String, mscorlib],[System.Object, mscorlib]], mscorlib"));
            w.ty(Some(STR));
            w.ty(Some(OBJ));
            w.num(0);
            w.num(*n);
            vars(&mut w);
        }
        w.finish()
    }

    fn sample() -> Vec<u8> {
        let mut root = W::default();
        root.int("Root", Some(INT), 3);
        let mut out = root.finish();
        out.extend(object("Global(Clone)", &[("GameState", 1, &|w: &mut W| w.var("Difficulty", |w| w.int("", Some("GameDifficulty, Assembly-CSharp"), 1)))]));
        out.extend(object(
            "Player_Sample(Clone)_1",
            &[
                ("CharacterStats", 3, &|w: &mut W| {
                    w.var("OverrideName", |w| w.string("", Some(STR), "Player"));
                    w.var("Experience", |w| w.int("", Some(INT), 8107));
                    w.var("BaseMight", |w| w.int("", Some(INT), 10));
                }),
                ("PlayerInventory", 2, &|w: &mut W| {
                    w.var("currencyTotalValue", |w| {
                        w.head(2, "", Some("CurrencyValue, Assembly-CSharp"));
                        w.num(1);
                        // A field of its declared type carries no type of its own.
                        w.head(6, "v", None);
                        w.body.push(1);
                        w.body.extend_from_slice(&4604f32.to_le_bytes());
                    });
                    w.var("ItemList", |w| {
                        w.head(1, "", Some("System.Collections.Generic.List`1[[InventoryItem, Assembly-CSharp]], mscorlib"));
                        w.ty(Some("InventoryItem, Assembly-CSharp"));
                        w.num(1);
                        w.int("Capacity", None, 4);
                        w.num(1);
                        w.head(2, "", None);
                        w.num(3);
                        w.string("BaseItem", Some(STR), "Assets/Data/Prefabs/Items/Ingredients/PLANT_Admeths_Wyrt.prefab");
                        w.int("StackSize", None, 6);
                        w.int("stackSize", None, 6);
                    });
                }),
            ],
        ));
        out.extend(object("Companion_GGP(Clone)_3", &[("CharacterStats", 1, &|w: &mut W| w.var("Experience", |w| w.int("", Some(INT), 500)))]));
        out
    }

    #[test]
    fn reads_and_edits_a_small_save() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        assert_eq!(doc.view.pointer("/Player/Money").and_then(Value::as_f64), Some(4604.0));
        assert_eq!(doc.view.pointer("/Game/Difficulty").unwrap(), "Normal");
        assert_eq!(doc.view.pointer("/Party/0/Name").unwrap(), "Player");
        assert_eq!(doc.view.pointer("/Party/1/Name").unwrap(), "Durance");
        assert_eq!(doc.view.pointer("/Player/Inventory/0/Item").unwrap(), "Admeths Wyrt");
        doc.set("/Player/Money", Value::from(99999)).unwrap();
        doc.set("/Party/0/Experience", Value::from(9000)).unwrap();
        doc.set("/Party/0/Might", Value::from(18)).unwrap();
        doc.set("/Player/Inventory/0/StackSize", Value::from(20)).unwrap();
        doc.set("/Game/Difficulty", Value::from("Path of the Damned")).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let back = parse(&out).unwrap();
        assert_eq!(back.view.pointer("/Player/Money").and_then(Value::as_f64), Some(99999.0));
        assert_eq!(back.view.pointer("/Party/0/Experience").unwrap(), 9000);
        assert_eq!(back.view.pointer("/Party/0/Might").unwrap(), 18);
        assert_eq!(back.view.pointer("/Player/Inventory/0/StackSize").unwrap(), 20);
        assert_eq!(back.view.pointer("/Game/Difficulty").unwrap(), "Path of the Damned");
        assert_eq!(difficulty_id(&out), Some("PathOfTheDamned"));
        assert!(doc.set("/Game/Difficulty", Value::from("Impossible")).is_err());
    }

    #[test]
    fn broken_or_cut_saves_are_refused() {
        let d = sample();
        for n in [d.len() - 1, d.len() / 2, 40] {
            assert!(parse(&d[..n]).is_err());
        }
        let mut more = d.clone();
        more.push(0);
        assert!(parse(&more).is_err());
        assert!(!looks(b"not a save at all, just some text here to fill it up"));
    }

    #[test]
    fn the_load_screen_difficulty_follows() {
        let xml = br#"<Simple name="Difficulty" type="GameDifficulty, Assembly-CSharp" value="Normal" />"#;
        let out = saveinfo_with_difficulty(xml, "Hard").unwrap();
        assert_eq!(out, br#"<Simple name="Difficulty" type="GameDifficulty, Assembly-CSharp" value="Hard" />"#.to_vec());
        assert!(saveinfo_with_difficulty(xml, "Normal").is_none());
    }

    /// `POE_SAVE=<file.savegame or MobileObjects.save> cargo test --release --lib pillars -- --ignored --nocapture`:
    /// the save opens through the full chain on its own screen and reads back unchanged.
    #[test]
    #[ignore]
    fn real_save_round_trips() {
        let Ok(p) = std::env::var("POE_SAVE") else { return };
        let raw = std::fs::read(&p).unwrap();
        let dec = crate::formats::decode_with(&raw, &crate::formats::Options::default());
        let crate::formats::Content::Tree(t) = &dec.content else { panic!("not a tree") };
        assert_eq!(t.kind_name, KIND);
        assert_eq!(crate::formats::encode(&dec.layers, &dec.content).unwrap(), raw);
        let mut v = t.view.clone();
        v.as_object_mut().unwrap().remove("Objects");
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    }
}

#[cfg(test)]
mod hostile {
    use super::*;

    #[test]
    fn deep_nesting_is_refused_without_running_out_of_stack() {
        let mut d = vec![1, 1, 1, 4, b'R', b'o', b'o', b't', 1, 1, 1, 22];
        d.extend_from_slice(b"System.Int32, mscorlib");
        d.extend_from_slice(&[6, 0, 0, 1, 1, 0, 0, 0]);
        // One object: names [""], types [Object], then objects of one property, nested on and on.
        d.extend_from_slice(&[1, 1, 1, 0, 1, 1, 1, 6]);
        d.extend_from_slice(b"Object");
        for _ in 0..200_000 {
            d.extend_from_slice(&[2, 0, 0, 1, 1]);
        }
        assert!(parse(&d).is_err());
    }
}
