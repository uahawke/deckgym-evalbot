use crate::models::{Attack, Card, EnergyType, StatusCondition, TrainerCard};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Main structure for following Game Tree design. Using "nesting" with a
/// SimpleAction to share common fields here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub actor: usize,
    pub action: SimpleAction,
    pub is_stack: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SimpleAction {
    DrawCard {
        amount: u8,
    },
    Play {
        trainer_card: TrainerCard,
    },

    // Card because of the fossil Trainer Cards...
    // usize is bench 1-based index, with 0 meaning Active pokemon, 1..4 meaning Bench
    Place(Card, usize),
    Evolve {
        evolution: Card,
        in_play_idx: usize,
        from_deck: bool,
    },
    UseAbility {
        in_play_idx: usize,
    },

    // Use the carried Attack definition as the current attack of the active Pokemon.
    // Carrying the whole Attack (instead of an index) lets a single codepath serve the
    // active's own attacks, copied attacks (e.g. Mew ex's Genome Hacking), and attacks
    // granted from previous evolutions (e.g. Celebi's Time Recall).
    Attack(Attack),
    // usize is in_play_pokemon index to retreat to. Can't Retreat(0)
    Retreat(usize),
    EndTurn,

    // Atomic actions as part of different effects.
    Attach {
        attachments: Vec<(u32, EnergyType, usize)>, // (amount, energy_type, in_play_idx)
        is_turn_energy: bool, // true if this is the energy from the zone that can be once per turn
    },
    MoveEnergy {
        from_in_play_idx: usize,
        to_in_play_idx: usize,
        energy_type: EnergyType,
        amount: u32,
    },
    AttachTool {
        in_play_idx: usize,
        tool_card: Card,
    },
    Heal {
        in_play_idx: usize,
        amount: u32,
        cure_status: bool,
    },
    HealAndDiscardEnergy {
        in_play_idx: usize,
        heal_amount: u32,
        discard_energies: Vec<EnergyType>,
    },
    MoveAllDamage {
        from: usize,
        to: usize,
    },
    ApplyDamage {
        attacking_ref: (usize, usize), // (attacking_player, attacking_pokemon_idx)
        targets: Vec<(u32, usize, usize)>, // Vec of (damage, target_player, in_play_idx)
        is_from_active_attack: bool,
    },
    ScheduleDelayedSpotDamage {
        target_player: usize,
        target_in_play_idx: usize,
        amount: u32,
    },
    /// Switch the in_play_idx pokemon with the active pokemon.
    Activate {
        player: usize,
        in_play_idx: usize,
    },
    // Custom Mechanics:
    /// Pokemon Communication: swap a specific Pokemon from hand with a random Pokemon from deck
    CommunicatePokemon {
        hand_pokemon: Card,
    },
    /// May: shuffle specific Pokemon from hand into your deck (no replacement)
    ShufflePokemonIntoDeck {
        hand_pokemon: Vec<Card>,
    },
    /// Maintenance: shuffle specific cards from hand into your deck, then draw a card
    ShuffleOwnCardsIntoDeck {
        cards: Vec<Card>,
    },
    /// Kid's Room: switch a specific card from hand with a random Pokemon Tool card from deck
    SwitchHandCardForRandomTool {
        hand_card: Card,
    },
    /// Silver: shuffle a specific Supporter from opponent's hand into their deck
    ShuffleOpponentSupporter {
        supporter_card: Card,
    },
    /// Mega Absol Ex: discard a specific Supporter from opponent's hand
    DiscardOpponentSupporter {
        supporter_card: Card,
    },
    /// Discard multiple specific cards from own hand
    DiscardOwnCards {
        cards: Vec<Card>,
    },
    /// Lusamine: attach energies from discard to a Pokemon
    AttachFromDiscard {
        in_play_idx: usize,
        num_random_energies: usize,
    },
    /// Volkner: attach a fixed number of a specific energy type from discard to a Pokemon
    AttachTypedFromDiscard {
        in_play_idx: usize,
        energy_type: EnergyType,
        count: usize,
    },
    /// Professor Sada: attach 3 specific different-typed energies from discard to Ancient Pokémon
    SadaAttach {
        assignments: Vec<(EnergyType, usize)>, // (energy_type, in_play_idx) × 3
    },
    /// Eevee Bag Option 1: Apply damage boost for Eevee evolutions this turn
    ApplyEeveeBagDamageBoost,
    /// Eevee Bag Option 2: Heal all Eevee evolutions
    HealAllEeveeEvolutions,
    /// Discard a Fossil from play (Fossils can be discarded at any time during your turn)
    DiscardFossil {
        in_play_idx: usize,
    },
    /// Vespiquen ex's Chase Order: discard 1 of your own Benched Pokémon, then deal the attack's
    /// boosted damage to the opponent's Active Pokémon. The two halves are one action so that the
    /// boosted damage is applied once — splitting it would apply Weakness (and other damage
    /// modifiers) to each half.
    DiscardOwnBenchedThenDamage {
        in_play_idx: usize,
        damage: u32,
    },
    /// Use an activated stadium effect (once per turn per player)
    UseStadium,
    /// Return a Pokemon in play to your hand (e.g., Ilima).
    ReturnPokemonToHand {
        in_play_idx: usize,
    },
    /// Shuffle a Pokemon from play into its owner's deck (e.g., Professor Turo).
    ShuffleInPlayPokemonIntoDeck {
        in_play_idx: usize,
    },
    /// Field Blower: discard the tool attached to a specific Pokémon (any player).
    DiscardToolFromPokemon {
        player: usize,
        in_play_idx: usize,
    },
    /// Field Blower: discard the active stadium.
    DiscardActiveStadium,
    /// Crawdaunt's Unruly Claw: discard a random Energy from the opponent's Active Pokémon
    DiscardRandomOpponentActiveEnergy,
    /// Psychic (Supporter): move a random Energy from one of the opponent's Benched Pokémon to
    /// the opponent's Active Pokémon.
    MoveRandomOpponentEnergyToActive {
        from_in_play_idx: usize,
    },
    /// Apply a chosen Special Condition to the opponent's Active Pokémon (e.g. Dustox's Select Powder).
    ApplyStatusToOpponentActive {
        condition: StatusCondition,
    },
    Noop, // No operation, used to have the user say "no" to a question
}

impl SimpleAction {
    /// Human-facing description, for a player-facing UI (the web frontend). Unlike `Display`
    /// (a debug-oriented dump used by the TUI and logs), this resolves card/energy names and
    /// board positions into plain English rather than printing the raw variant/field structure.
    pub fn describe(&self) -> String {
        fn slot(idx: usize) -> String {
            if idx == 0 {
                "Active".to_string()
            } else {
                format!("Bench {idx}")
            }
        }

        match self {
            SimpleAction::DrawCard { amount } => {
                if *amount == 1 {
                    "Draw a card".to_string()
                } else {
                    format!("Draw {amount} cards")
                }
            }
            SimpleAction::Play { trainer_card } => format!("Play {}", trainer_card.name),
            SimpleAction::Place(card, index) => {
                format!("Place {} ({})", card.get_name(), slot(*index))
            }
            SimpleAction::Evolve {
                evolution,
                in_play_idx,
                ..
            } => format!("Evolve into {} ({})", evolution.get_name(), slot(*in_play_idx)),
            SimpleAction::UseAbility { in_play_idx } => {
                format!("Use ability ({})", slot(*in_play_idx))
            }
            SimpleAction::Attack(attack) => format!("Attack: {}", attack.title),
            SimpleAction::Retreat(index) => format!("Retreat to {}", slot(*index)),
            SimpleAction::EndTurn => "End turn".to_string(),
            SimpleAction::Attach { attachments, .. } => {
                let parts = attachments
                    .iter()
                    .map(|(amount, energy_type, in_play_idx)| {
                        format!("{amount}x {energy_type} \u{2192} {}", slot(*in_play_idx))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("Attach {parts}")
            }
            SimpleAction::MoveEnergy {
                from_in_play_idx,
                to_in_play_idx,
                energy_type,
                amount,
            } => format!(
                "Move {amount}x {energy_type} energy from {} to {}",
                slot(*from_in_play_idx),
                slot(*to_in_play_idx)
            ),
            SimpleAction::AttachTool {
                in_play_idx,
                tool_card,
            } => format!(
                "Attach {} to {}",
                tool_card.get_name(),
                slot(*in_play_idx)
            ),
            SimpleAction::Heal {
                in_play_idx,
                amount,
                cure_status,
            } => {
                if *cure_status {
                    format!("Heal {amount} and cure status ({})", slot(*in_play_idx))
                } else {
                    format!("Heal {amount} damage ({})", slot(*in_play_idx))
                }
            }
            SimpleAction::HealAndDiscardEnergy {
                in_play_idx,
                heal_amount,
                discard_energies,
            } => {
                let energies = discard_energies
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "Heal {heal_amount} and discard {energies} energy ({})",
                    slot(*in_play_idx)
                )
            }
            SimpleAction::MoveAllDamage { from, to } => {
                format!("Move all damage from {} to {}", slot(*from), slot(*to))
            }
            SimpleAction::ApplyDamage { .. } => "Resolve damage".to_string(),
            SimpleAction::ScheduleDelayedSpotDamage {
                target_in_play_idx,
                amount,
                ..
            } => format!(
                "Schedule {amount} delayed damage to {}",
                slot(*target_in_play_idx)
            ),
            SimpleAction::Activate { in_play_idx, .. } => {
                format!("Move {} to Active", slot(*in_play_idx))
            }
            SimpleAction::CommunicatePokemon { hand_pokemon } => format!(
                "Trade {} for a random Pokémon from your deck",
                hand_pokemon.get_name()
            ),
            SimpleAction::ShufflePokemonIntoDeck { hand_pokemon } => {
                let names = hand_pokemon
                    .iter()
                    .map(|c| c.get_name())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("Shuffle {names} into your deck")
            }
            SimpleAction::ShuffleOwnCardsIntoDeck { cards } => {
                format!("Shuffle {} card(s) into your deck and draw", cards.len())
            }
            SimpleAction::SwitchHandCardForRandomTool { hand_card } => format!(
                "Trade {} for a random Tool from your deck",
                hand_card.get_name()
            ),
            SimpleAction::ShuffleOpponentSupporter { supporter_card } => format!(
                "Shuffle opponent's {} into their deck",
                supporter_card.get_name()
            ),
            SimpleAction::DiscardOpponentSupporter { supporter_card } => {
                format!("Discard opponent's {}", supporter_card.get_name())
            }
            SimpleAction::DiscardOwnCards { cards } => {
                let names = cards
                    .iter()
                    .map(|c| c.get_name())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("Discard {names}")
            }
            SimpleAction::AttachFromDiscard {
                in_play_idx,
                num_random_energies,
            } => format!(
                "Attach {num_random_energies} random energy from discard to {}",
                slot(*in_play_idx)
            ),
            SimpleAction::AttachTypedFromDiscard {
                in_play_idx,
                energy_type,
                count,
            } => format!(
                "Attach {count}x {energy_type} energy from discard to {}",
                slot(*in_play_idx)
            ),
            SimpleAction::SadaAttach { assignments } => {
                let parts = assignments
                    .iter()
                    .map(|(energy_type, idx)| format!("{energy_type} \u{2192} {}", slot(*idx)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("Attach energy from discard: {parts}")
            }
            SimpleAction::ApplyEeveeBagDamageBoost => {
                "Boost damage for Eevee evolutions this turn".to_string()
            }
            SimpleAction::HealAllEeveeEvolutions => "Heal all Eevee evolutions".to_string(),
            SimpleAction::DiscardFossil { in_play_idx } => {
                format!("Discard Fossil ({})", slot(*in_play_idx))
            }
            SimpleAction::DiscardOwnBenchedThenDamage { in_play_idx, damage } => {
                format!("Discard {} to deal {damage} damage", slot(*in_play_idx))
            }
            SimpleAction::UseStadium => "Use stadium".to_string(),
            SimpleAction::ReturnPokemonToHand { in_play_idx } => {
                format!("Return {} to hand", slot(*in_play_idx))
            }
            SimpleAction::ShuffleInPlayPokemonIntoDeck { in_play_idx } => {
                format!("Shuffle {} into deck", slot(*in_play_idx))
            }
            SimpleAction::DiscardToolFromPokemon { in_play_idx, .. } => {
                format!("Discard tool from {}", slot(*in_play_idx))
            }
            SimpleAction::DiscardActiveStadium => "Discard the active stadium".to_string(),
            SimpleAction::DiscardRandomOpponentActiveEnergy => {
                "Discard a random Energy from opponent's Active".to_string()
            }
            SimpleAction::MoveRandomOpponentEnergyToActive { from_in_play_idx } => format!(
                "Move a random Energy from opponent's {} to their Active",
                slot(*from_in_play_idx)
            ),
            SimpleAction::ApplyStatusToOpponentActive { condition } => {
                format!("Apply {condition:?} to opponent's Active")
            }
            SimpleAction::Noop => "Pass".to_string(),
        }
    }

    /// Where this action's button belongs in a card-oriented UI (the web frontend), as a
    /// `(hand_card_id, in_play_idx)` hint: at most one is set, meaning "render this action on the
    /// hand card with this id" or "render it on the acting player's in-play slot at this index"
    /// respectively. `(None, None)` means there's no single natural card for it (`EndTurn`, a
    /// choice spanning multiple cards, or an effect that can target the *opponent's* side, which
    /// this deliberately excludes since `in_play_idx` alone can't say whose board it's on).
    ///
    /// Purely a UI placement hint -- `describe()` remains the source of truth for what the action
    /// actually does, since a hint here is necessarily approximate (e.g. two duplicate hand cards
    /// share an id and so share whatever buttons match that id).
    pub fn target_hint(&self) -> (Option<String>, Option<usize>) {
        match self {
            SimpleAction::Play { trainer_card } => (Some(trainer_card.id.clone()), None),
            SimpleAction::Place(card, _) => (Some(card.get_id()), None),
            SimpleAction::Evolve { in_play_idx, .. } => (None, Some(*in_play_idx)),
            SimpleAction::UseAbility { in_play_idx } => (None, Some(*in_play_idx)),
            SimpleAction::Attack(_) => (None, Some(0)), // only the active Pokemon can attack
            SimpleAction::Retreat(idx) => (None, Some(*idx)),
            SimpleAction::Attach { attachments, .. } => (
                None,
                attachments.first().map(|(_, _, idx)| *idx),
            ),
            SimpleAction::AttachTool { in_play_idx, .. } => (None, Some(*in_play_idx)),
            SimpleAction::Heal { in_play_idx, .. } => (None, Some(*in_play_idx)),
            SimpleAction::HealAndDiscardEnergy { in_play_idx, .. } => (None, Some(*in_play_idx)),
            SimpleAction::AttachFromDiscard { in_play_idx, .. } => (None, Some(*in_play_idx)),
            SimpleAction::AttachTypedFromDiscard { in_play_idx, .. } => {
                (None, Some(*in_play_idx))
            }
            SimpleAction::DiscardFossil { in_play_idx } => (None, Some(*in_play_idx)),
            SimpleAction::DiscardOwnBenchedThenDamage { in_play_idx, .. } => {
                (None, Some(*in_play_idx))
            }
            SimpleAction::ReturnPokemonToHand { in_play_idx } => (None, Some(*in_play_idx)),
            SimpleAction::ShuffleInPlayPokemonIntoDeck { in_play_idx } => {
                (None, Some(*in_play_idx))
            }
            SimpleAction::CommunicatePokemon { hand_pokemon } => {
                (Some(hand_pokemon.get_id()), None)
            }
            SimpleAction::SwitchHandCardForRandomTool { hand_card } => {
                (Some(hand_card.get_id()), None)
            }
            // Everything else either spans multiple cards (a Vec<Card> choice), can target the
            // opponent's board (e.g. DiscardToolFromPokemon's `player` field), or has no card at
            // all (EndTurn, DrawCard, UseStadium, Noop) -- left for the general action list.
            _ => (None, None),
        }
    }
}

impl fmt::Display for SimpleAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimpleAction::DrawCard { amount } => write!(f, "DrawCard({amount})"),
            SimpleAction::Play { trainer_card } => write!(f, "Play({trainer_card:?})"),
            SimpleAction::Place(card, index) => write!(f, "Place({card}, {index})"),
            SimpleAction::Evolve {
                evolution,
                in_play_idx,
                from_deck,
            } => {
                write!(
                    f,
                    "Evolve({evolution}, {in_play_idx}, from_deck: {from_deck})"
                )
            }
            SimpleAction::UseAbility { in_play_idx } => write!(f, "UseAbility({in_play_idx})"),
            SimpleAction::Attack(attack) => write!(f, "Attack({})", attack.title),
            SimpleAction::Retreat(index) => write!(f, "Retreat({index})"),
            SimpleAction::EndTurn => write!(f, "EndTurn"),
            SimpleAction::Attach {
                attachments,
                is_turn_energy,
            } => {
                let attachments_str = attachments
                    .iter()
                    .map(|(amount, energy_type, in_play_idx)| {
                        format!("({amount}, {energy_type:?}, {in_play_idx})")
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "Attach({attachments_str:?}, {is_turn_energy})")
            }
            SimpleAction::MoveEnergy {
                from_in_play_idx,
                to_in_play_idx,
                energy_type,
                amount,
            } => {
                write!(
                    f,
                    "MoveEnergy(from:{from_in_play_idx}, to:{to_in_play_idx}, {amount}x {energy_type:?})"
                )
            }
            SimpleAction::AttachTool {
                in_play_idx,
                tool_card,
            } => {
                write!(f, "AttachTool({in_play_idx}, {})", tool_card.get_name())
            }
            SimpleAction::Heal {
                in_play_idx,
                amount,
                cure_status,
            } => write!(f, "Heal({in_play_idx}, {amount}, cure:{cure_status})"),
            SimpleAction::HealAndDiscardEnergy {
                in_play_idx,
                heal_amount,
                discard_energies,
            } => write!(
                f,
                "HealAndDiscardEnergy({in_play_idx}, {heal_amount}, {discard_energies:?})"
            ),
            SimpleAction::MoveAllDamage { from, to } => {
                write!(f, "MoveAllDamage(from:{from}, to:{to})")
            }
            SimpleAction::ApplyDamage {
                attacking_ref,
                targets,
                is_from_active_attack,
            } => {
                let targets_str = targets
                    .iter()
                    .map(|(damage, target_player, in_play_idx)| {
                        format!("({damage}, {target_player}, {in_play_idx})")
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(
                    f,
                    "ApplyDamage(attacker:{:?}, targets:[{}], from_active:{})",
                    attacking_ref, targets_str, is_from_active_attack
                )
            }
            SimpleAction::ScheduleDelayedSpotDamage {
                target_player,
                target_in_play_idx,
                amount,
            } => write!(
                f,
                "ScheduleDelayedSpotDamage(target:{target_player}:{target_in_play_idx}, amount:{amount})"
            ),
            SimpleAction::Activate {
                player,
                in_play_idx,
            } => write!(f, "Activate({player}, {in_play_idx})"),
            SimpleAction::CommunicatePokemon { hand_pokemon } => {
                write!(f, "CommunicatePokemon({hand_pokemon})")
            }
            SimpleAction::ShufflePokemonIntoDeck { hand_pokemon } => {
                write!(f, "ShufflePokemonIntoDeck({:?})", hand_pokemon)
            }
            SimpleAction::ShuffleOwnCardsIntoDeck { cards } => {
                write!(f, "ShuffleOwnCardsIntoDeck({:?})", cards)
            }
            SimpleAction::SwitchHandCardForRandomTool { hand_card } => {
                write!(f, "SwitchHandCardForRandomTool({hand_card})")
            }
            SimpleAction::ShuffleOpponentSupporter { supporter_card } => {
                write!(f, "ShuffleOpponentSupporter({supporter_card})")
            }
            SimpleAction::DiscardOpponentSupporter { supporter_card } => {
                write!(f, "DiscardOpponentSupporter({supporter_card})")
            }
            SimpleAction::DiscardOwnCards { cards } => {
                write!(f, "DiscardOwnCards({:?})", cards)
            }
            SimpleAction::AttachFromDiscard {
                in_play_idx,
                num_random_energies,
            } => {
                write!(f, "AttachFromDiscard({in_play_idx}, {num_random_energies})")
            }
            SimpleAction::AttachTypedFromDiscard {
                in_play_idx,
                energy_type,
                count,
            } => {
                write!(f, "AttachTypedFromDiscard({in_play_idx}, {energy_type:?}, {count})")
            }
            SimpleAction::SadaAttach { assignments } => {
                let s = assignments
                    .iter()
                    .map(|(e, idx)| format!("{e:?}→{idx}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "SadaAttach([{s}])")
            }
            SimpleAction::ApplyEeveeBagDamageBoost => {
                write!(f, "ApplyEeveeBagDamageBoost")
            }
            SimpleAction::HealAllEeveeEvolutions => {
                write!(f, "HealAllEeveeEvolutions")
            }
            SimpleAction::DiscardFossil { in_play_idx } => {
                write!(f, "DiscardFossil({in_play_idx})")
            }
            SimpleAction::DiscardOwnBenchedThenDamage {
                in_play_idx,
                damage,
            } => {
                write!(f, "DiscardOwnBenchedThenDamage({in_play_idx}, {damage})")
            }
            SimpleAction::ReturnPokemonToHand { in_play_idx } => {
                write!(f, "ReturnPokemonToHand({in_play_idx})")
            }
            SimpleAction::ShuffleInPlayPokemonIntoDeck { in_play_idx } => {
                write!(f, "ShuffleInPlayPokemonIntoDeck({in_play_idx})")
            }
            SimpleAction::DiscardToolFromPokemon { player, in_play_idx } => {
                write!(f, "DiscardToolFromPokemon({player}, {in_play_idx})")
            }
            SimpleAction::DiscardActiveStadium => write!(f, "DiscardActiveStadium"),
            SimpleAction::DiscardRandomOpponentActiveEnergy => {
                write!(f, "DiscardRandomOpponentActiveEnergy")
            }
            SimpleAction::MoveRandomOpponentEnergyToActive { from_in_play_idx } => {
                write!(f, "MoveRandomOpponentEnergyToActive({from_in_play_idx})")
            }
            SimpleAction::UseStadium => write!(f, "UseStadium"),
            SimpleAction::ApplyStatusToOpponentActive { condition } => {
                write!(f, "ApplyStatusToOpponentActive({condition:?})")
            }
            SimpleAction::Noop => write!(f, "Noop"),
        }
    }
}
