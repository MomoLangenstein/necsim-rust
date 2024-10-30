use necsim_core::{
    cogs::{Habitat, MathsCore},
    landscape::Location,
};

use necsim_core_bond::NonNegativeF64;

use crate::array2d::Array2D;

use super::InMemoryDispersalSamplerError;

#[expect(clippy::module_name_repetitions)]
pub fn check_in_memory_dispersal_contract<M: MathsCore, H: Habitat<M>>(
    dispersal: &Array2D<NonNegativeF64>,
    habitat: &H,
) -> Result<(), InMemoryDispersalSamplerError> {
    let habitat_extent = habitat.get_extent();

    let habitat_area = usize::from(habitat_extent.width()) * usize::from(habitat_extent.height());

    if dispersal.num_rows() != habitat_area || dispersal.num_columns() != habitat_area {
        return Err(InMemoryDispersalSamplerError::DispersalMapSizeMismatch);
    }

    let habitat_width = habitat_extent.width();

    for row_index in 0..dispersal.num_rows() {
        #[expect(clippy::cast_possible_truncation)]
        let dispersal_origin = Location::new(
            (row_index % usize::from(habitat_width)) as u32,
            (row_index / usize::from(habitat_width)) as u32,
        );

        if habitat.get_habitat_at_location(&dispersal_origin) > 0 {
            let mut any_dispersal = false;

            for col_index in 0..dispersal.num_columns() {
                #[expect(clippy::cast_possible_truncation)]
                let dispersal_target = Location::new(
                    (col_index % usize::from(habitat_width)) as u32,
                    (col_index / usize::from(habitat_width)) as u32,
                );

                if dispersal[(row_index, col_index)] > 0.0_f64 {
                    if habitat.get_habitat_at_location(&dispersal_target) == 0 {
                        return Err(InMemoryDispersalSamplerError::DispersalToNonHabitat);
                    }

                    any_dispersal = true;
                }
            }

            if !any_dispersal {
                return Err(InMemoryDispersalSamplerError::NoDispersalFromHabitat);
            }
        } else {
            for col_index in 0..dispersal.num_columns() {
                if dispersal[(row_index, col_index)] != 0.0_f64 {
                    // Dispersal probability from non-habitat must be 0.0
                    // - Dispersal from non-habitat (> 0.0)
                    // - Dispersal probabilities must be non-negative (< 0.0)
                    return Err(InMemoryDispersalSamplerError::DispersalFromNonHabitat);
                }
            }
        }
    }

    Ok(())
}
