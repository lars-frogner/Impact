//! Equirectangular mapping of direction-dependent data.

use crate::{io, physics::fph};
use anyhow::Result;
use impact_math::{Angle, Float, Radians};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

/// A map of values as a function of all directions. The directions are
/// discretized onto a 2D grid using an equirectangular projection (meaning the
/// grid coordinates are the spherical azimuthal angle phi and polar angle
/// theta).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EquirectangularMap<V> {
    values: Vec<V>,
    n_phi_coords: usize,
    n_theta_coords: usize,
    grid_cell_size: fph,
    inverse_grid_cell_size: fph,
}

impl<V: Clone + Default> EquirectangularMap<V> {
    /// Creates a new map with default values using the given number of
    /// increments for the full range [0, pi] of polar angles theta. The number
    /// of azimuthal angles phi, with range [0, 2*pi], will be double this,
    /// yielding the same grid cell extent for both angles.
    ///
    /// # Panics
    /// If the given number of theta coordinates is zero.
    pub fn empty(n_theta_coords: usize) -> Self {
        assert_ne!(n_theta_coords, 0);

        let n_phi_coords = 2 * n_theta_coords;
        let n_values = n_phi_coords * n_theta_coords;

        let grid_cell_size = fph::PI / (n_theta_coords as fph);
        let inverse_grid_cell_size = 1.0 / grid_cell_size;

        Self {
            values: vec![V::default(); n_values],
            n_phi_coords,
            n_theta_coords,
            grid_cell_size,
            inverse_grid_cell_size,
        }
    }

    /// Returns the number of theta (polar) coordinates used for the map.
    pub fn n_theta_coords(&self) -> usize {
        self.n_theta_coords
    }

    /// Returns the number of phi (azimuthal) coordinates used for the map.
    pub fn n_phi_coords(&self) -> usize {
        self.n_phi_coords
    }

    /// Returns the angular extent of a single grid cell. The extent is the same
    /// along both the azimuthal and polar axis.
    pub fn grid_cell_size(&self) -> Radians<fph> {
        Radians(self.grid_cell_size)
    }

    /// Returns a reference to the value at the given spherical coordinates (phi
    /// is the azimuthal angle and theta is the polar angle).
    pub fn value<A: Angle<fph>>(&self, phi: A, theta: A) -> &V {
        let idx = self.compute_linear_idx_from_angles(phi, theta);
        &self.values[idx]
    }

    /// Returns a mutable reference to the value at the given spherical
    /// coordinates (phi is the azimuthal angle and theta is the polar angle).
    pub fn value_mut<A: Angle<fph>>(&mut self, phi: A, theta: A) -> &mut V {
        let idx = self.compute_linear_idx_from_angles(phi, theta);
        &mut self.values[idx]
    }

    /// Returns a reference to the value at the given indices.
    pub fn value_at_indices_mut(&mut self, phi_idx: usize, theta_idx: usize) -> &mut V {
        let idx = self.compute_linear_idx(phi_idx, theta_idx);
        &mut self.values[idx]
    }

    /// Computes the index corresponding to the given phi (azimuthal) angle. Any
    /// value for the
    pub fn compute_phi_idx<A: Angle<fph>>(&self, phi: A) -> usize {
        self.compute_idx(phi.radians().rem_euclid(fph::TWO_PI))
    }

    /// Computes the index corresponding to the given theta (polar) angle.
    pub fn compute_theta_idx<A: Angle<fph>>(&self, theta: A) -> usize {
        let mut theta = theta.radians().rem_euclid(fph::TWO_PI);
        if theta > fph::PI {
            theta = fph::TWO_PI - theta;
        }
        // Prevent roundoff errors from producing a too large index
        usize::min(self.n_theta_coords - 1, self.compute_idx(theta))
    }

    /// Given a quadratic region of spherical coordinates in the map centered at
    /// the given angles and with the given half extent, returns an iterator
    /// over the indices and angular distance from the region's center for each
    /// grid cell in the region. The returned iterator always contains
    /// information for at least one grid cell (the one containing the center).
    ///
    /// # Panics
    /// If the given half extent does not exceed zero.
    pub fn find_angle_indices_and_angular_distances_for_region<A: Angle<fph>>(
        &self,
        center_phi: A,
        center_theta: A,
        region_half_extent: A,
    ) -> impl Iterator<Item = (usize, usize, Radians<fph>)> + use<A, V> {
        let center_phi = center_phi.radians();
        let center_theta = center_theta.radians();
        let region_half_extent = region_half_extent.radians();

        assert!(region_half_extent > 0.0);

        let half_grid_cell_size = 0.5 * self.grid_cell_size;

        let region_half_extent = fph::max(half_grid_cell_size, region_half_extent);

        let n_angles_across = (2.0 * region_half_extent / self.grid_cell_size).ceil() as usize;

        let start_phi = center_phi - region_half_extent + half_grid_cell_size;
        let start_theta = center_theta - region_half_extent + half_grid_cell_size;

        #[allow(clippy::needless_collect)]
        let theta_values: Vec<_> = (0..n_angles_across)
            .map(|region_theta_idx| {
                let theta = start_theta + (region_theta_idx as fph) * self.grid_cell_size;
                let theta_idx = self.compute_theta_idx(Radians(theta));
                let (sin_theta, cos_theta) = theta.sin_cos();
                (sin_theta, cos_theta, theta_idx)
            })
            .collect();

        let phi_values: Vec<_> = (0..n_angles_across)
            .map(|region_phi_idx| {
                let phi = start_phi + (region_phi_idx as fph) * self.grid_cell_size;
                let phi_idx = self.compute_phi_idx(Radians(phi));
                (phi, phi_idx)
            })
            .collect();

        let (sin_center_theta, cos_center_theta) = center_theta.sin_cos();

        theta_values
            .into_iter()
            .flat_map(move |(sin_theta, cos_theta, theta_idx)| {
                phi_values.clone().into_iter().map(move |(phi, phi_idx)| {
                    let angular_distance = fph::acos(
                        sin_center_theta * sin_theta
                            + cos_center_theta * cos_theta * fph::cos(phi - center_phi),
                    );
                    (phi_idx, theta_idx, Radians(angular_distance))
                })
            })
    }

    /// Converts this map into a new map where the given mapping function has
    /// been applied to each value in the map.
    pub fn map_values<VNEW>(self, mapping: impl Fn(V) -> VNEW) -> EquirectangularMap<VNEW> {
        let Self {
            values,
            n_phi_coords,
            n_theta_coords,
            grid_cell_size,
            inverse_grid_cell_size,
        } = self;

        let mapped_values = values.into_iter().map(mapping).collect();

        EquirectangularMap {
            values: mapped_values,
            n_phi_coords,
            n_theta_coords,
            grid_cell_size,
            inverse_grid_cell_size,
        }
    }

    fn compute_linear_idx_from_angles<A: Angle<fph>>(&self, phi: A, theta: A) -> usize {
        let phi_idx = self.compute_phi_idx(phi);
        let theta_idx = self.compute_theta_idx(theta);
        self.compute_linear_idx(phi_idx, theta_idx)
    }

    fn compute_idx(&self, angle: fph) -> usize {
        (angle * self.inverse_grid_cell_size).floor() as usize
    }

    fn compute_linear_idx(&self, phi_idx: usize, theta_idx: usize) -> usize {
        theta_idx * self.n_phi_coords + phi_idx
    }
}

impl<D: Serialize + DeserializeOwned> EquirectangularMap<D> {
    /// Serializes the map into the `MessagePack` format and saves it at the
    /// given path.
    pub fn save_to_file(&self, output_file_path: impl AsRef<Path>) -> Result<()> {
        let byte_buffer = bincode::serialize(self)?;
        io::util::save_data_as_binary(output_file_path, &byte_buffer)?;
        Ok(())
    }

    /// Loads and returns the `MessagePack` serialized map at the given path.
    pub fn read_from_file(file_path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        let table = bincode::deserialize(&buffer)?;
        Ok(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_compute_correct_phi_idx() {
        let map = EquirectangularMap::<fph>::empty(3);
        let half_grid_cell_size = map.grid_cell_size() * 0.5;
        assert_eq!(map.compute_phi_idx(half_grid_cell_size), 0);
        assert_eq!(map.compute_phi_idx(half_grid_cell_size * 3.0), 1);
        assert_eq!(
            map.compute_phi_idx(half_grid_cell_size * -1.0),
            map.n_phi_coords() - 1
        );
        assert_eq!(
            map.compute_phi_idx(Radians(fph::TWO_PI) + half_grid_cell_size),
            0
        );
        assert!(map.compute_phi_idx(Radians(0.0)) < map.n_phi_coords());
        assert!(map.compute_phi_idx(Radians(fph::TWO_PI)) < map.n_phi_coords());
    }

    #[test]
    fn should_compute_correct_theta_idx() {
        let map = EquirectangularMap::<fph>::empty(3);
        let half_grid_cell_size = map.grid_cell_size() * 0.5;
        assert_eq!(map.compute_theta_idx(half_grid_cell_size), 0);
        assert_eq!(map.compute_theta_idx(half_grid_cell_size * -1.0), 0);
        assert_eq!(map.compute_theta_idx(half_grid_cell_size * 3.0), 1);
        assert_eq!(
            map.compute_theta_idx(Radians(fph::PI) - half_grid_cell_size),
            map.n_theta_coords() - 1
        );
        assert_eq!(
            map.compute_theta_idx(Radians(fph::PI) + half_grid_cell_size),
            map.n_theta_coords() - 1
        );
        assert!(map.compute_theta_idx(Radians(0.0)) < map.n_theta_coords());
        assert!(map.compute_theta_idx(Radians(fph::PI)) < map.n_theta_coords());
    }
}
