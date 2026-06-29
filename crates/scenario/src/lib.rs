mod dto;

use std::collections::HashSet;
use std::path::Path;

use world::{
    Axis, Beliefs, Channel, ChannelId, ChannelKind, Claim, Culture, DisclosureRule, Fact,
    Objective, Organization, Persona, PersonaState, Personality, Player, Policy, Pretext,
    Relationship, RunStatus, Secret, Sensitivity, SuccessCondition, Suspicion, Trust, Tuning,
    Voice, World,
};

pub use dto::ScenarioPackage;

#[derive(Debug, thiserror::Error)]
pub enum ScenarioError {
    #[error("reading scenario {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("parsing scenario: {0}")]
    Parse(#[from] ron::error::SpannedError),
    #[error(transparent)]
    Lint(#[from] LintError),
}

/// Every way an authored package can be malformed. Reported at load — never mid-run.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LintError {
    #[error("duplicate persona id: {0}")]
    DuplicatePersona(String),
    #[error("duplicate secret id: {0}")]
    DuplicateSecret(String),
    #[error("secret {secret} is owned by unknown persona {owner}")]
    DanglingSecretOwner { secret: String, owner: String },
    #[error("fact {fact} is owned by unknown persona {owner}")]
    DanglingFactOwner { fact: String, owner: String },
    #[error("persona {persona} knows unknown secret {secret}")]
    DanglingKnowledge { persona: String, secret: String },
    #[error("persona {persona} references unknown fact {fact}")]
    DanglingFactRef { persona: String, fact: String },
    #[error("persona {persona} reports to unknown persona {target}")]
    DanglingReportsTo { persona: String, target: String },
    #[error("persona {persona} has a relationship to unknown persona {other}")]
    DanglingRelationship { persona: String, other: String },
    #[error("objective references unknown secret {0}")]
    UnknownObjectiveSecret(String),
    #[error("objective secret {secret} is unreachable: {reason}")]
    Unreachable { secret: String, reason: String },
}

pub fn load(path: &Path) -> Result<World, ScenarioError> {
    let text = std::fs::read_to_string(path).map_err(|source| ScenarioError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let package = ron::from_str::<ScenarioPackage>(&text)?;
    Ok(instantiate(package)?)
}

pub fn instantiate(package: ScenarioPackage) -> Result<World, LintError> {
    lint(&package)?;

    let org = Organization {
        name: package.organization.name,
        industry: package.organization.industry,
        culture: Culture {
            formality: package.organization.formality.into(),
            security_conscious: package.organization.security_conscious,
        },
        policies: package
            .organization
            .policies
            .into_iter()
            .map(|p| Policy {
                id: p.id.into(),
                description: p.description,
                forbids_disclosure_of: p.forbids_disclosure_of.into(),
            })
            .collect(),
    };

    let personas = package.cast.into_iter().map(persona).collect();
    let secrets = package.secrets.into_iter().map(secret).collect();
    let facts = package
        .facts
        .into_iter()
        .map(|f| Fact {
            id: f.id.into(),
            owner: f.owner.into(),
            text: f.text,
        })
        .collect();

    let dto::SuccessDto::SecretDisclosed(target) = package.objective.success;
    let success = SuccessCondition::SecretDisclosed(target.into());

    let player = Player {
        pretext: package.seed.pretext.map(|p| Pretext {
            label: p.label,
            claimed_identity: p.claimed_identity,
            internal_claim: p.internal_claim,
            verifiable: p.verifiable,
        }),
        intel: package
            .seed
            .intel
            .into_iter()
            .map(|c| Claim {
                key: c.key,
                value: c.value,
            })
            .collect(),
        disclosed: Vec::new(),
    };

    let tuning = package
        .tuning
        .map_or_else(Tuning::default, |t| t.merge(Tuning::default()));

    Ok(World {
        org,
        personas,
        secrets,
        facts,
        channels: vec![Channel {
            id: ChannelId::from("dm"),
            kind: ChannelKind::Messenger,
            messages: Vec::new(),
        }],
        player,
        objective: Objective {
            description: package.objective.description,
            success,
            turn_budget: package.objective.turn_budget,
        },
        tuning,
        clock: 0,
        org_suspicion: Suspicion::new(0),
        status: RunStatus::Active,
        log: world::CausalLog::default(),
    })
}

fn persona(p: dto::PersonaDto) -> Persona {
    Persona {
        id: p.id.into(),
        name: p.name,
        title: p.title,
        department: p.department,
        reports_to: p.reports_to.map(Into::into),
        personality: Personality {
            agreeableness: Axis::new(p.personality.agreeableness),
            conscientiousness: Axis::new(p.personality.conscientiousness),
            security_awareness: Axis::new(p.personality.security_awareness),
            busyness: Axis::new(p.personality.busyness),
            ego: Axis::new(p.personality.ego),
            helpfulness: Axis::new(p.personality.helpfulness),
        },
        voice: Voice {
            style: p.voice.style,
            formality: p.voice.formality.into(),
            avg_words: p.voice.avg_words,
            quirks: p.voice.quirks,
            emoji: p.voice.emoji,
        },
        knowledge: p.knowledge.into_iter().map(Into::into).collect(),
        facts: p.facts.into_iter().map(Into::into).collect(),
        relationships: p
            .relationships
            .into_iter()
            .map(|r| Relationship {
                other: r.other.into(),
                trust: Trust::new(r.trust),
                would_warn: r.would_warn,
                defers_to: r.defers_to,
            })
            .collect(),
        hooks: p.hooks,
        red_lines: p.red_lines.into_iter().map(Into::into).collect(),
        state: PersonaState {
            suspicion: Suspicion::new(p.start.suspicion),
            trust: Trust::new(p.start.trust),
            mood: p.start.mood.into(),
            beliefs: Beliefs::default(),
            principle_history: Vec::new(),
        },
    }
}

fn secret(s: dto::SecretDto) -> Secret {
    Secret {
        id: s.id.into(),
        owner: s.owner.into(),
        kind: s.kind.into(),
        sensitivity: Sensitivity::new(s.sensitivity),
        value: s.value,
        disclosure: DisclosureRule {
            trust_min: Trust::new(s.disclosure.trust_min),
            suspicion_max: Suspicion::new(s.disclosure.suspicion_max),
            required_pretext: s.disclosure.required_pretext,
            requires_authorization: s.disclosure.requires_authorization,
        },
    }
}

fn lint(pkg: &ScenarioPackage) -> Result<(), LintError> {
    let mut persona_ids = HashSet::new();
    for p in &pkg.cast {
        if !persona_ids.insert(p.id.as_str()) {
            return Err(LintError::DuplicatePersona(p.id.clone()));
        }
    }

    let mut secret_ids = HashSet::new();
    for s in &pkg.secrets {
        if !secret_ids.insert(s.id.as_str()) {
            return Err(LintError::DuplicateSecret(s.id.clone()));
        }
    }
    let fact_ids: HashSet<&str> = pkg.facts.iter().map(|f| f.id.as_str()).collect();

    for s in &pkg.secrets {
        if !persona_ids.contains(s.owner.as_str()) {
            return Err(LintError::DanglingSecretOwner {
                secret: s.id.clone(),
                owner: s.owner.clone(),
            });
        }
    }
    for f in &pkg.facts {
        if !persona_ids.contains(f.owner.as_str()) {
            return Err(LintError::DanglingFactOwner {
                fact: f.id.clone(),
                owner: f.owner.clone(),
            });
        }
    }

    for p in &pkg.cast {
        if let Some(target) = &p.reports_to {
            if !persona_ids.contains(target.as_str()) {
                return Err(LintError::DanglingReportsTo {
                    persona: p.id.clone(),
                    target: target.clone(),
                });
            }
        }
        for secret in &p.knowledge {
            if !secret_ids.contains(secret.as_str()) {
                return Err(LintError::DanglingKnowledge {
                    persona: p.id.clone(),
                    secret: secret.clone(),
                });
            }
        }
        for fact in &p.facts {
            if !fact_ids.contains(fact.as_str()) {
                return Err(LintError::DanglingFactRef {
                    persona: p.id.clone(),
                    fact: fact.clone(),
                });
            }
        }
        for rel in &p.relationships {
            if !persona_ids.contains(rel.other.as_str()) {
                return Err(LintError::DanglingRelationship {
                    persona: p.id.clone(),
                    other: rel.other.clone(),
                });
            }
        }
    }

    check_reachable(pkg)
}

/// The objective must be winnable: the target secret exists, isn't blocked by a red
/// line or an org policy, its required pretext is the one the player starts with, and
/// its suspicion ceiling is above the owner's starting suspicion (suspicion only rises).
fn check_reachable(pkg: &ScenarioPackage) -> Result<(), LintError> {
    let dto::SuccessDto::SecretDisclosed(target) = &pkg.objective.success;

    let Some(secret) = pkg.secrets.iter().find(|s| &s.id == target) else {
        return Err(LintError::UnknownObjectiveSecret(target.clone()));
    };

    let unreachable = |reason: &str| LintError::Unreachable {
        secret: target.clone(),
        reason: reason.to_owned(),
    };

    if pkg
        .organization
        .policies
        .iter()
        .any(|p| p.forbids_disclosure_of == secret.kind)
    {
        return Err(unreachable("an org policy forbids disclosing this kind"));
    }

    let owner = pkg
        .cast
        .iter()
        .find(|p| p.id == secret.owner)
        .expect("owner existence checked above");
    if owner.red_lines.contains(&secret.kind) {
        return Err(unreachable("the owner red-lines this kind of disclosure"));
    }

    if let Some(required) = &secret.disclosure.required_pretext {
        let provided = pkg
            .seed
            .pretext
            .as_ref()
            .is_some_and(|p| &p.label == required);
        if !provided {
            return Err(unreachable(
                "no seed pretext matches the required pretext label",
            ));
        }
    }

    if owner.start.suspicion >= secret.disclosure.suspicion_max {
        return Err(unreachable(
            "owner starts at or above the suspicion ceiling",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests;
