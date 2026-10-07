//! Events: everything that happened as a result of a command.
//!
//! The frontend turns events into log messages and animations.
//! Logic code never builds display strings.

use crate::boons::Boon;
use crate::content::KindId;
use crate::dread::DreadBand;
use crate::geom::Point;
use crate::item::{Burden, ItemKindId, Potency, SideEffect, TinctureEffect};
use crate::map::Tile;
use crate::rites::{RiteFailure, RiteId};
use crate::skills::{Skill, Technique};

/// Someone in a fight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Who {
    Player,
    Monster(KindId),
}

/// What killed the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    Attack(KindId),
    HeavyBlow(KindId),
    Fire,
    /// A chanted rite.
    Chant(KindId),
    /// Falling through rotten boards.
    Fall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The player moved to a new tile.
    PlayerMoved {
        to: Point,
    },
    /// The player tried to move but something solid was in the way. Costs no time.
    PlayerBlocked {
        at: Point,
        tile: Tile,
    },
    /// The player waited a turn.
    PlayerWaited,
    /// A landmark came into view for the first time on this floor.
    Spotted {
        tile: Tile,
        at: Point,
    },
    /// The player went down to a new floor.
    Descended {
        depth: u8,
    },
    /// The player tried to use stairs that aren't there. Costs no time.
    NoStairsHere,
    /// The way back up is closed during the descent. Costs no time.
    StairsSealed {
        depth: u8,
    },
    /// A run or rest was refused because something hostile is in view. Costs no time.
    RunRefused,
    /// Rest was asked for with nothing to recover. Costs no time.
    NothingToRest,
    /// A rest ended after this many turns.
    Rested {
        turns: u32,
    },

    /// Tallow came into view for the first time.
    SpottedTallow {
        at: Point,
    },
    /// The player picked up tallow.
    TallowFound {
        amount: u32,
    },
    CandleLow,
    CandleGuttering,
    CandleBurnedOut,
    CandleSnuffed,
    CandleLit,
    /// Tried to light a candle with no tallow left. Costs no time.
    CandleSpent,
    /// A creature's bite put the candle out and ate tallow.
    CandleEaten {
        kind: KindId,
        amount: u32,
    },

    /// Dread crossed into a new band.
    DreadChanged {
        band: DreadBand,
    },
    /// Something only the uneasy hear. `seed` picks the line.
    Whisper {
        seed: u32,
    },
    /// A phantom came apart. `struck` if the player swung at it.
    PhantomFaded {
        kind: KindId,
        struck: bool,
    },
    /// Dread took form near the player.
    Manifested,
    /// The Manifestation was killed. Dread eases.
    ManifestationBanished,
    /// The player left the floor with the Manifestation still on it. Dread eases.
    ManifestationEscaped,

    /// An item came into view for the first time.
    SpottedItem {
        kind: ItemKindId,
        at: Point,
    },
    NothingToPickUp,
    PickedUp {
        kind: ItemKindId,
        count: u32,
    },
    /// No pack slot left for another kind of thing; it stays on the floor.
    PackFull {
        kind: ItemKindId,
    },
    Dropped {
        kind: ItemKindId,
        count: u32,
    },
    Equipped {
        kind: ItemKindId,
    },
    Unequipped {
        kind: ItemKindId,
    },
    /// That item has no use of its own. Costs no time.
    CantUse {
        kind: ItemKindId,
    },
    /// A tincture was drunk. `learned` if this is the first of its kind.
    Drank {
        kind: ItemKindId,
        effect: TinctureEffect,
        potency: Potency,
        side: SideEffect,
        amount: u32,
        learned: bool,
    },
    /// That item can't be thrown. Costs no time.
    CantThrow {
        kind: ItemKindId,
    },
    /// You can't aim at yourself. Costs no time.
    BadTarget,
    Thrown {
        kind: ItemKindId,
    },
    Fired {
        kind: ItemKindId,
    },
    /// Nothing to shoot with. Costs no time.
    NoRangedWeapon,
    /// No ammunition of this kind. Costs no time.
    NoAmmo {
        kind: ItemKindId,
    },
    /// A projectile reached a creature. `damage` is `None` on a miss.
    ProjectileHit {
        item: ItemKindId,
        target: KindId,
        damage: Option<u32>,
    },
    /// A breakable projectile shattered.
    Shattered {
        kind: ItemKindId,
        at: Point,
    },
    /// Carrying more or less crossed a weight threshold.
    BurdenChanged {
        burden: Burden,
    },
    /// Too heavy to move. Costs no time.
    TooHeavy,

    SkillRankUp {
        skill: Skill,
        rank: u32,
    },
    TechniqueLearned {
        skill: Skill,
        technique: Technique,
    },
    /// A new level. A boon draft is waiting.
    LevelUp {
        level: u32,
    },
    BoonTaken {
        boon: Boon,
    },
    /// A creature missed you and you struck back at once.
    Riposte {
        kind: KindId,
    },
    /// A bludgeon blow cost a creature its next action.
    Staggered {
        kind: KindId,
    },
    /// A missile pinned a creature in place.
    Pinned {
        kind: KindId,
    },

    /// The first time this run the player sees a kind of creature.
    FirstSighting {
        kind: KindId,
    },
    /// A monster noticed the player. `seen` is whether the player can see it.
    Noticed {
        kind: KindId,
        seen: bool,
    },
    /// A monster spoke. `line` indexes its barks.
    Bark {
        kind: KindId,
        line: usize,
    },
    /// A blow was struck. `damage` is `None` on a miss.
    Attack {
        attacker: Who,
        defender: Who,
        damage: Option<u32>,
    },
    /// A monster raised a heavy blow aimed at `target`. It lands next action.
    WindUp {
        kind: KindId,
        target: Point,
    },
    /// A heavy blow came down on `target`. `damage` is `None` if nobody was there.
    HeavyBlow {
        kind: KindId,
        target: Point,
        damage: Option<u32>,
    },
    /// A monster lost its nerve.
    Fled {
        kind: KindId,
    },
    MonsterDied {
        kind: KindId,
        at: Point,
    },
    PlayerDied {
        cause: Cause,
    },

    /// Study or rendering asked for with no body underfoot. Costs no time.
    NoCorpse,
    /// This kind has nothing more to teach you. Costs no time.
    NothingToLearn {
        kind: KindId,
    },
    /// Finished studying a body. `first` if this kind is new to you.
    Studied {
        kind: KindId,
        first: bool,
    },
    /// Study or rendering stopped early; the progress is kept.
    WorkInterrupted {
        kind: KindId,
    },
    Rendered {
        kind: KindId,
        tallow: u32,
    },
    /// A body has begun to swell. It will rot soon.
    CorpseSwelling {
        kind: KindId,
        at: Point,
    },
    /// A rotting body burst into a fly swarm.
    CorpseHatched {
        kind: KindId,
        at: Point,
    },
    /// A text was read. `learned` if it taught a new rite.
    Read {
        kind: ItemKindId,
        learned: bool,
    },
    ReadingInterrupted {
        kind: ItemKindId,
    },
    RiteLearned {
        rite: RiteId,
    },
    /// A rite couldn't be cast. Costs no time and no dread.
    RiteFailed {
        rite: RiteId,
        why: RiteFailure,
    },
    Cast {
        rite: RiteId,
    },
    Compelled {
        kind: KindId,
    },
    /// Compel wore off. The creature remembers.
    CompelEnded {
        kind: KindId,
    },
    Knelt {
        kind: KindId,
    },
    Leeched {
        kind: KindId,
        amount: u32,
    },
    Transferred {
        kind: KindId,
        amount: u32,
    },
    EyesBorrowed {
        kind: KindId,
    },
    EyesReturned,
    Unseen {
        kind: KindId,
    },
    FalseFlameLit {
        at: Point,
    },
    FalseFlameOut,
    /// A lantern-eater ate the False Flame.
    FlameEaten {
        kind: KindId,
    },
    Shrouded,
    ShroudFaded,
    Sanctified,
    SanctityFaded,
    /// One of the Taken was freed and ran.
    Exorcised {
        kind: KindId,
        at: Point,
    },
    /// Too strong to cast out; it burns instead.
    ExorciseResisted {
        kind: KindId,
        damage: u32,
    },
    /// One of the Taken stepped on holy ground and lost an action.
    Flinched {
        kind: KindId,
    },
    /// You traded places with your thrall.
    SwappedPlaces {
        kind: KindId,
    },

    /// A bell rang out at `at`. Creatures nearby will come to look.
    BellRung {
        at: Point,
    },
    /// Shelves caught fire.
    FireSpread {
        at: Point,
        tile: Tile,
    },
    /// The last fire on the floor burned out.
    FireOut,
    Burned {
        who: Who,
        damage: u32,
    },
    /// Walking into fire needs a second step in the same direction. Costs no time.
    FireAhead {
        at: Point,
    },
    /// A thrown flask set fire where it broke.
    Ignited {
        at: Point,
    },
    OilSpilled {
        at: Point,
    },
    Slipped {
        who: Who,
    },
    DoorOpened {
        at: Point,
    },
    DoorShut {
        at: Point,
    },
    /// No open door beside you. Costs no time.
    NoDoorToClose,
    /// Something is in the doorway. Costs no time.
    DoorBlocked,
    /// You broke your own seal by opening the door.
    SealBroken {
        at: Point,
    },
    SealFaded {
        at: Point,
    },
    /// A cold brazier needs a lit candle. Costs no time.
    NeedFlame,
    BrazierLit {
        at: Point,
    },
    /// Holy water left the ground holy.
    Consecrated {
        at: Point,
    },
    Turned {
        kind: KindId,
    },
    Beckoned {
        kind: KindId,
    },
    Exchanged {
        kind: KindId,
    },
    Hushed,
    HushFaded,
    Sealed {
        at: Point,
    },
    Banished {
        kind: KindId,
    },

    /// Wading into deep water needs a second step. Costs no time.
    DeepWaterAhead {
        at: Point,
    },
    /// Deep water put your candle out.
    CandleDrowned,
    /// No lighting a candle while standing in deep water. Costs no time.
    CantLightInWater,
    /// Stepping on rotten boards needs a second step. Costs no time.
    RottenAhead {
        at: Point,
    },
    /// The boards gave way and you fell to the floor below.
    Fell {
        depth: u8,
        damage: u32,
    },
    Blinded {
        kind: KindId,
    },
    SightReturns,
    /// A creature began chanting at you. It lands next action if it can still see you.
    ChantBegins {
        kind: KindId,
    },
    ChantLands {
        kind: KindId,
        damage: u32,
    },
    /// You broke its line of sight in time.
    ChantBroken {
        kind: KindId,
    },
    Dragged {
        kind: KindId,
    },
    /// A creature is singing at you; dread rises.
    Singing {
        kind: KindId,
    },
    /// A creature burst into another when it died.
    Burst {
        kind: KindId,
        into: KindId,
    },
    Summoned {
        kind: KindId,
        into: KindId,
    },
    /// A creature raised a body as something else.
    Raised {
        kind: KindId,
        body: KindId,
        into: KindId,
    },
    /// A boss locked the doors around it.
    DoorsLocked {
        kind: KindId,
    },
    BooksIgnited {
        kind: KindId,
    },
    /// The door is locked fast and will give in time. Costs no time.
    DoorLocked,
    /// A mini-boss is down.
    BossDefeated {
        kind: KindId,
    },
}
