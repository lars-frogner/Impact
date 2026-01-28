module [
    compute_sphere_mass,
    compute_sphere_mass_density,
    compute_sphere_emissive_luminance,
]

compute_sphere_mass = |radius, mass_density|
    (4.0 / 3.0) * Num.pi * Num.pow(radius, 3) * mass_density

compute_sphere_mass_density = |radius, mass|
    mass / ((4.0 / 3.0) * Num.pi * Num.pow(radius, 3))

compute_sphere_emissive_luminance = |luminous_intensity, radius|
    luminous_intensity / (Num.pi * Num.pow(radius, 2))
