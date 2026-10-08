//! Events: everything that happened as a result of a command.
//!
//! The frontend turns events into log messages and animations.
//! Logic code never builds display strings.

use crate::boons::Boon;
use crate::content::KindId;
use crate::dread::DreadBand;
use crate::geom::Point;
use crate::item::{Burden, ItemKindId, Potency, SideEffect, TinctureEffect};
use crate::leavings::{AnomalyKind, LeavingId};
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
    /// A Leaving's price.
    Leaving,
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
    /// Your dread takes form. `grown`: how many times it has come before.
    Manifested {
        grown: u32,
    },
    /// Something made of your fear is on its way. It arrives in a few turns
    /// unless you calm down.
    NightmareComing,
    NightmareArrives {
        kind: KindId,
    },
    /// You calmed before it came.
    NightmareTurnedAway,
    /// You are calm: it comes apart.
    NightmareFades {
        kind: KindId,
    },
    /// A phantom reached you. Not real, but fear hurts: health, or more dread.
    PhantomBit {
        kind: KindId,
        damage: Option<u32>,
    },
    /// Your blow did half: its kind shrugs off this sort of weapon (said once a kind).
    Resisted {
        kind: KindId,
    },
    /// Your sickle opened a wound that bleeds.
    Bleeding {
        kind: KindId,
    },
    /// Your censer's best blow set the ground under it alight.
    Kindled {
        kind: KindId,
    },
    /// Your hook hauled it a step closer.
    Hooked {
        kind: KindId,
    },
    /// Your staff drove it a step back.
    Shoved {
        kind: KindId,
    },
    /// Candlelight scatters it.
    Scatters {
        kind: KindId,
    },
    /// Badly hurt, it runs to mend.
    Retreats {
        kind: KindId,
    },
    /// It struck at you from range.
    Volley {
        kind: KindId,
        damage: Option<u32>,
    },
    /// It crouches to leap at the marked tile.
    Crouches {
        kind: KindId,
        target: Point,
    },
    Leaps {
        kind: KindId,
        damage: Option<u32>,
    },
    /// It leapt where you no longer were.
    LeapsShort {
        kind: KindId,
    },
    /// You brace yourself.
    Guarding,
    NothingToShove,
    TooBigToShove {
        kind: KindId,
    },
    ShoveBlocked {
        kind: KindId,
    },
    YouShove {
        kind: KindId,
    },
    /// Shoved onto rotten boards, it went through them.
    ShovedThrough {
        kind: KindId,
    },
    /// It has hold of you.
    Grabbed {
        kind: KindId,
    },
    /// You can't step away while it holds you.
    HeldFast {
        kind: KindId,
    },
    /// A blow that feeds your dread.
    FedOnFear {
        kind: KindId,
        dread: u32,
    },
    /// It came close and your candle went out.
    CandleSnuffedBy {
        kind: KindId,
    },
    /// It sent a false copy of itself.
    Doubled {
        kind: KindId,
    },
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
    /// You began rendering, and the smell drew something.
    RenderSmell,
    /// One of the Dreaming came apart and left grave-wax.
    WaxLeft {
        kind: KindId,
        amount: u32,
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
    /// A rite was learned with every slot full: forget one, or let it go.
    RiteOffered {
        rite: RiteId,
    },
    /// Forgotten to make room for another.
    RiteForgotten {
        rite: RiteId,
    },
    /// An offered rite was let go.
    RiteLetGo {
        rite: RiteId,
    },
    /// A level brought room for another rite.
    RiteSlotGained {
        slots: u32,
    },
    /// A studied body is spoiled for rendering.
    BodySpoiled {
        kind: KindId,
    },
    /// You carry all the Leavings you can; this one stays where it lies.
    LeavingsFull {
        id: LeavingId,
    },
    /// Walking into the Following again will push through it, at a price.
    PushThroughAhead {
        at: Point,
    },
    /// You forced your way past it.
    PushedThrough {
        kind: KindId,
        damage: u32,
    },
    /// The Vigil Candle flared and drove it back.
    Flared {
        kind: KindId,
    },
    /// The candle has flared already on this floor.
    FlareSpent,
    /// Nothing is close enough for the flare to drive back.
    NothingToFlare,
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
    /// You gave dread to a lit brazier and it mended you.
    Offered {
        dread: u32,
        health: u32,
    },
    /// The brazier wants dread, and you have none to give.
    NothingToOffer,
    /// You are already whole; the brazier takes nothing.
    WholeAlready,
    /// Your light was seen: something new has come onto the floor to find it.
    LightDrawn,
    /// Letters on a wall, seen only in the dark.
    WritingSpotted {
        at: Point,
    },
    /// You traced the letters in the dark.
    WritingRead {
        learned: bool,
    },
    /// Candlelight washes the letters out; they can only be read in the dark.
    WritingWashedOut,
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

    /// A seep room came into view for the first time.
    SeepSpotted,
    /// A Leaving came into view for the first time.
    SpottedLeaving {
        id: LeavingId,
    },
    /// You took a Leaving. `first` the first time this run: a warning.
    LeavingTaken {
        id: LeavingId,
        first: bool,
    },
    LeavingDropped {
        id: LeavingId,
    },
    /// It doesn't wake when you will it. Costs no time.
    LeavingWontWake {
        id: LeavingId,
    },
    /// Used too recently. Costs no time.
    LeavingResting {
        id: LeavingId,
    },
    /// Its next price would kill you. Drop it, or it takes it next turn.
    LeavingThreatens {
        id: LeavingId,
    },
    /// A Leaving woke. `learned` the first time: its rule is known now.
    LeavingWoke {
        id: LeavingId,
        learned: bool,
    },
    Duplicated {
        kind: ItemKindId,
    },
    DarkSightFaded,
    /// Something thrown showed where an anomaly is.
    AnomalyRevealed {
        kind: AnomalyKind,
        at: Point,
    },
    /// You walked into an anomaly.
    AnomalyStruck {
        kind: AnomalyKind,
        damage: u32,
    },
    /// Walking into a known anomaly needs a second step. Costs no time.
    AnomalyAhead {
        at: Point,
    },

    /// A blow passes through something that cannot die.
    Undying {
        kind: KindId,
    },
    /// The swarm coat took the blow.
    SwarmAbsorbs {
        left: u32,
    },
    /// Light or fire ate at the swarm coat.
    SwarmThins {
        left: u32,
    },
    /// The first phase is over: he leaves his body.
    LordLeavesBody,
    /// Flies gather over a body: he will rise in it next turn.
    LordGathers {
        at: Point,
    },
    LordPossesses {
        at: Point,
    },
    /// A rite pinned him in the body he wears.
    LordBound,
    /// The third phase: the Lord of Flies himself.
    LordRises,
    LordFalls,
    /// The candle can't be taken while he lives. Costs no time.
    CandleGuarded,
    VigilTaken,
    Ascended {
        floor: u8,
    },
    ReachedChurch,
    /// The Following is close behind.
    FollowingNear,
    FollowingArrives,
    /// You have nothing to set on the altar. Costs no time.
    AltarEmpty,
    /// The way down is behind you now. Costs no time.
    NoGoingBack,
    Won,
    /// Auto-explore found nothing left to see. Costs no time.
    Explored,
    /// A creature that sweeps heaves itself up after a blow: an opening.
    Recovering {
        kind: KindId,
    },
    /// You couldn't carry it all: this much tallow spilled at your feet.
    TallowSpilled {
        amount: u32,
    },
}
