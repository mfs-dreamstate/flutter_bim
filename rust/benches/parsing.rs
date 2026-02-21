use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

/// Generate a synthetic IFC STEP file string with the given number of wall entities.
///
/// Produces a valid STEP header followed by a DATA section containing
/// IFCPROJECT, IFCSITE, IFCBUILDING, and `entity_count` IFCWALL entries
/// (each with a minimal representation referencing placeholder IFCPRODUCTDEFINITIONSHAPE).
fn generate_test_ifc(entity_count: usize) -> String {
    let mut lines = Vec::with_capacity(entity_count + 30);

    // STEP header
    lines.push("ISO-10303-21;".to_string());
    lines.push("HEADER;".to_string());
    lines.push(
        "FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');".to_string(),
    );
    lines.push(
        "FILE_NAME('bench.ifc','2024-01-01T00:00:00',(''),(''),'','','');".to_string(),
    );
    lines.push("FILE_SCHEMA(('IFC2X3'));".to_string());
    lines.push("ENDSEC;".to_string());
    lines.push("DATA;".to_string());

    // A minimal project / site / building so the parser finds something
    lines.push("#1=IFCPROJECT('0001',#2,'BenchProject',$,$,$,$,$,$);".to_string());
    lines.push("#2=IFCOWNERHISTORY($,$,$,$,$,$,$,$);".to_string());
    lines.push("#3=IFCSITE('0002',$,'Site',$,$,$,$,$,$,$,$,$,$);".to_string());
    lines.push("#4=IFCBUILDING('0003',$,'Building',$,$,$,$,$,$,$,$,$);".to_string());

    let base_id = 100;
    for i in 0..entity_count {
        let eid = base_id + i * 3;
        let global_id = format!("W{:06}", i);
        let name = format!("Wall_{}", i);
        // IFCWALL(GlobalId, OwnerHistory, Name, Description, ObjectType,
        //         ObjectPlacement, Representation, Tag)
        lines.push(format!(
            "#{}=IFCWALL('{}',#2,'{}',$,$,$,$,$);",
            eid, global_id, name
        ));
        // A trivial cartesian point (used to pad the entity count)
        lines.push(format!(
            "#{}=IFCCARTESIANPOINT(({:.1},{:.1},0.0));",
            eid + 1,
            i as f64 * 3.0,
            0.0
        ));
        // A trivial property set (to exercise lookup paths)
        lines.push(format!(
            "#{}=IFCPROPERTYSET('PS{:06}',$,'Pset_WallCommon',$,());",
            eid + 2, i
        ));
    }

    lines.push("ENDSEC;".to_string());
    lines.push("END-ISO-10303-21;".to_string());
    lines.join("\n")
}

fn bench_ifc_parse(c: &mut Criterion) {
    let small_ifc = generate_test_ifc(100);
    let medium_ifc = generate_test_ifc(1000);
    let large_ifc = generate_test_ifc(5000);

    let mut group = c.benchmark_group("ifc_parsing");
    for (name, content) in [
        ("100_entities", &small_ifc),
        ("1000_entities", &medium_ifc),
        ("5000_entities", &large_ifc),
    ] {
        group.bench_with_input(
            BenchmarkId::new("parse", name),
            content,
            |b, content| {
                b.iter(|| rust::bim::ifc_parser::IfcFile::parse(content).unwrap());
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_ifc_parse);
criterion_main!(benches);
