use std::path::PathBuf;

use world::PersonaId;

/// A model honoring structured output, but wrapping it in a markdown fence and prose —
/// the exact shape that was silently riding the inert fallback. Proves the tolerant
/// parser recovers the intent and the referee then produces non-zero deltas end to end.
#[test]
fn markdown_wrapped_analyst_output_moves_the_needle() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/it-support-credential/scenario.ron");
    let world = scenario::load(&path).expect("starter scenario loads");
    let contact = PersonaId::from("priya_v");
    let persona = world.persona(&contact).expect("priya exists");

    let raw = "Here is my classification:\n\
        ```json\n\
        {\n\
          \"principles\": [\"liking\", \"reciprocity\"],\n\
          \"claims\": [{\"key\": \"office\", \"value\": \"Houston\"}],\n\
          \"authority_claim\": null,\n\
          \"ask\": null,\n\
          \"out_of_world\": false\n\
        }\n\
        ```\n\
        Let me know if you need anything else.";

    let action = llm::try_parse_analyst(raw).expect("tolerant parser recovers fenced JSON");
    assert!(!action.principles.is_empty());

    let appraisal = referee::appraise(&world, persona, &action);
    assert!(
        appraisal.trust_delta > 0,
        "a recognized rapport play must move trust, not stay inert"
    );
    assert!(
        !appraisal.reasons.is_empty(),
        "the causal log must record why the deltas moved"
    );
}
