module [
    compute_sphere_mass,
    compute_sphere_mass_density,
    compute_sphere_emissive_luminance,
    compute_sphere_luminous_intensity,
]

compute_sphere_mass = |radius, mass_density|
    compute_sphere_volume(radius) * mass_density

compute_sphere_mass_density = |radius, mass|
    mass / compute_sphere_volume(radius)

compute_sphere_emissive_luminance = |luminous_intensity, radius|
    luminous_intensity / compute_disk_area(radius)

compute_sphere_luminous_intensity = |emissive_luminance, radius|
    emissive_luminance * compute_disk_area(radius)

compute_sphere_volume = |radius|
    (4.0 / 3.0) * Num.pi * Num.pow(radius, 3)

compute_disk_area = |radius|
    Num.pi * Num.pow(radius, 2)
