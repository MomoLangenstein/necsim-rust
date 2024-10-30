use alloc::vec::Vec;

use necsim_core::lineage::{GlobalLineageReference, Lineage};

#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum ExceptionalLineage {
    Coalescence {
        child: Lineage,
        parent: GlobalLineageReference,
    },
    OutOfDeme(Lineage),
    OutOfHabitat(Lineage),
}

pub struct SplitExceptionalLineages {
    pub coalescence: Vec<(Lineage, GlobalLineageReference)>,
    pub out_of_deme: Vec<Lineage>,
    pub out_of_habitat: Vec<Lineage>,
}

impl ExceptionalLineage {
    #[must_use]
    pub fn split_vec(exceptional_lineages: Vec<ExceptionalLineage>) -> SplitExceptionalLineages {
        let mut coalescence_lineages = Vec::new();
        let mut out_of_deme_lineages = Vec::new();
        let mut out_of_habitat_lineages = Vec::new();

        for lineage in exceptional_lineages {
            match lineage {
                ExceptionalLineage::Coalescence { child, parent } => {
                    coalescence_lineages.push((child, parent));
                },
                ExceptionalLineage::OutOfDeme(lineage) => out_of_deme_lineages.push(lineage),
                ExceptionalLineage::OutOfHabitat(lineage) => out_of_habitat_lineages.push(lineage),
            }
        }

        SplitExceptionalLineages {
            coalescence: coalescence_lineages,
            out_of_deme: out_of_deme_lineages,
            out_of_habitat: out_of_habitat_lineages,
        }
    }

    pub fn drain_coalescing_lineages(
        exceptional_lineages: &mut Vec<ExceptionalLineage>,
    ) -> impl Iterator<Item = Lineage> + '_ {
        exceptional_lineages
            .extract_if(|exceptional_lineage| {
                matches!(exceptional_lineage, ExceptionalLineage::Coalescence { .. })
            })
            .map(|exceptional_lineage| match exceptional_lineage {
                ExceptionalLineage::Coalescence { child: lineage, .. }
                | ExceptionalLineage::OutOfDeme(lineage)
                | ExceptionalLineage::OutOfHabitat(lineage) => lineage,
            })
    }
}
