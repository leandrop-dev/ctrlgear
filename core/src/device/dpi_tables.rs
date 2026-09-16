//! Shared DPI-value -> device-value mapping tables for sensors used by more
//! than one device family.
//!
//! Each table is transcribed directly from rivalcfg (WTFPL) via `gh api`
//! (`rivalcfg/devices/dpi/truemove_core.py`, `truemove_air.py`), not
//! re-derived or guessed, and cross-checked programmatically against the
//! fetched source for entry count and boundary values before being pasted
//! here.

/// TrueMove Core sensor DPI mapping: 200-8500 DPI in 100-DPI steps, used by
/// the Aerox 3 (`rivalcfg/devices/aerox3.py`). Also used by the Rival 3 Gen
/// 2's sensor, though that device's actual `sensitivity` command
/// (`multidpi_range_choice_xy`, command `0x34`) encodes an independent X/Y
/// DPI pair per preset — a shape [`crate::device::profile::DpiProfile`]
/// doesn't support, so [`crate::device::aerox_gen2_rgb::RIVAL3_GEN2_PROFILE`]
/// leaves `dpi: None` rather than emit a wrong-shaped command; see that
/// module's doc comment.
pub const TRUEMOVE_CORE_MAPPING: &[(u16, u16)] = &[
    (200, 0x04), (300, 0x06), (400, 0x08), (500, 0x0B),
    (600, 0x0D), (700, 0x0F), (800, 0x12), (900, 0x14),
    (1000, 0x16), (1100, 0x19), (1200, 0x1B), (1300, 0x1D),
    (1400, 0x20), (1500, 0x22), (1600, 0x24), (1700, 0x27),
    (1800, 0x29), (1900, 0x2B), (2000, 0x2E), (2100, 0x30),
    (2200, 0x32), (2300, 0x34), (2400, 0x37), (2500, 0x39),
    (2600, 0x3B), (2700, 0x3E), (2800, 0x40), (2900, 0x42),
    (3000, 0x45), (3100, 0x47), (3200, 0x49), (3300, 0x4C),
    (3400, 0x4E), (3500, 0x50), (3600, 0x53), (3700, 0x55),
    (3800, 0x57), (3900, 0x5A), (4000, 0x5C), (4100, 0x5E),
    (4200, 0x61), (4300, 0x63), (4400, 0x65), (4500, 0x68),
    (4600, 0x6A), (4700, 0x6C), (4800, 0x6F), (4900, 0x71),
    (5000, 0x73), (5100, 0x76), (5200, 0x78), (5300, 0x7A),
    (5400, 0x7D), (5500, 0x7F), (5600, 0x81), (5700, 0x84),
    (5800, 0x86), (5900, 0x88), (6000, 0x8B), (6100, 0x8D),
    (6200, 0x8F), (6300, 0x92), (6400, 0x94), (6500, 0x96),
    (6600, 0x99), (6700, 0x9B), (6800, 0x9D), (6900, 0xA0),
    (7000, 0xA2), (7100, 0xA4), (7200, 0xA7), (7300, 0xA9),
    (7400, 0xAB), (7500, 0xAD), (7600, 0xB0), (7700, 0xB2),
    (7800, 0xB4), (7900, 0xB7), (8000, 0xB9), (8100, 0xBC),
    (8200, 0xBE), (8300, 0xC0), (8400, 0xC3), (8500, 0xC5),
];

/// TrueMove Air sensor DPI mapping: 100-18000 DPI in 100-DPI steps, used by
/// the Aerox 5 (`rivalcfg/devices/aerox5.py`, which imports
/// `dpi.truemove_air` — *not* `truemove_core`, correcting an earlier
/// assumption in this feature's task notes that Aerox 5 shared Aerox 3's
/// sensor/range). Verified independently against `dpi/truemove_air.py`
/// rather than reused from [`crate::device::rival3_wireless`]'s own
/// `DPI_MAPPING`: that table (used by the already-shipped, hardware-verified
/// Rival 3 Wireless) was found during this task's research to diverge from
/// rivalcfg's real `truemove_air.choices` in ~165 of its 180 entries (values
/// consistently 1 device-unit higher from ~1200 DPI upward). That divergence
/// predates this task, is out of its scope (Rival 3 Wireless is
/// hardware-verified and unrelated to the Aerox/Prime Mini family being
/// added here), and is left untouched — flagged instead for separate
/// review.
pub const TRUEMOVE_AIR_MAPPING: &[(u16, u16)] = &[
    (100, 0x00), (200, 0x02), (300, 0x03), (400, 0x04),
    (500, 0x05), (600, 0x06), (700, 0x07), (800, 0x09),
    (900, 0x0A), (1000, 0x0B), (1100, 0x0C), (1200, 0x0D),
    (1300, 0x0E), (1400, 0x10), (1500, 0x11), (1600, 0x12),
    (1700, 0x13), (1800, 0x14), (1900, 0x16), (2000, 0x17),
    (2100, 0x18), (2200, 0x19), (2300, 0x1A), (2400, 0x1B),
    (2500, 0x1D), (2600, 0x1E), (2700, 0x1F), (2800, 0x20),
    (2900, 0x21), (3000, 0x23), (3100, 0x25), (3200, 0x26),
    (3300, 0x27), (3400, 0x28), (3500, 0x29), (3600, 0x2A),
    (3700, 0x2C), (3800, 0x2D), (3900, 0x2E), (4000, 0x2F),
    (4100, 0x30), (4200, 0x32), (4300, 0x33), (4400, 0x34),
    (4500, 0x35), (4600, 0x36), (4700, 0x38), (4800, 0x39),
    (4900, 0x3A), (5000, 0x3B), (5100, 0x3C), (5200, 0x3E),
    (5300, 0x3F), (5400, 0x40), (5500, 0x41), (5600, 0x42),
    (5700, 0x44), (5800, 0x45), (5900, 0x46), (6000, 0x47),
    (6100, 0x48), (6200, 0x4A), (6300, 0x4B), (6400, 0x4C),
    (6500, 0x4D), (6600, 0x4E), (6700, 0x50), (6800, 0x51),
    (6900, 0x52), (7000, 0x53), (7100, 0x54), (7200, 0x56),
    (7300, 0x57), (7400, 0x58), (7500, 0x59), (7600, 0x5A),
    (7700, 0x5C), (7800, 0x5D), (7900, 0x5E), (8000, 0x5F),
    (8100, 0x60), (8200, 0x62), (8300, 0x63), (8400, 0x64),
    (8500, 0x65), (8600, 0x66), (8700, 0x68), (8800, 0x69),
    (8900, 0x6A), (9000, 0x6B), (9100, 0x6C), (9200, 0x6E),
    (9300, 0x6F), (9400, 0x70), (9500, 0x71), (9600, 0x72),
    (9700, 0x74), (9800, 0x75), (9900, 0x76), (10000, 0x77),
    (10100, 0x78), (10200, 0x7A), (10300, 0x7B), (10400, 0x7C),
    (10500, 0x7D), (10600, 0x7E), (10700, 0x80), (10800, 0x81),
    (10900, 0x82), (11000, 0x83), (11100, 0x84), (11200, 0x86),
    (11300, 0x87), (11400, 0x88), (11500, 0x89), (11600, 0x8A),
    (11700, 0x8C), (11800, 0x8D), (11900, 0x8E), (12000, 0x8F),
    (12100, 0x90), (12200, 0x92), (12300, 0x93), (12400, 0x94),
    (12500, 0x95), (12600, 0x96), (12700, 0x98), (12800, 0x99),
    (12900, 0x9A), (13000, 0x9B), (13100, 0x9C), (13200, 0x9E),
    (13300, 0x9F), (13400, 0xA0), (13500, 0xA1), (13600, 0xA2),
    (13700, 0xA4), (13800, 0xA5), (13900, 0xA6), (14000, 0xA7),
    (14100, 0xA8), (14200, 0xAA), (14300, 0xAB), (14400, 0xAC),
    (14500, 0xAD), (14600, 0xAE), (14700, 0xB0), (14800, 0xB1),
    (14900, 0xB2), (15000, 0xB3), (15100, 0xB4), (15200, 0xB5),
    (15300, 0xB6), (15400, 0xB7), (15500, 0xB8), (15600, 0xB9),
    (15700, 0xBA), (15800, 0xBB), (15900, 0xBC), (16000, 0xBD),
    (16100, 0xBF), (16200, 0xC0), (16300, 0xC2), (16400, 0xC3),
    (16500, 0xC4), (16600, 0xC5), (16700, 0xC6), (16800, 0xC7),
    (16900, 0xC9), (17000, 0xCA), (17100, 0xCB), (17200, 0xCC),
    (17300, 0xCD), (17400, 0xCF), (17500, 0xD0), (17600, 0xD1),
    (17700, 0xD2), (17800, 0xD3), (17900, 0xD5), (18000, 0xD6),
];

/// Prime Mini DPI mapping: 50-18000 DPI in 50-DPI steps
/// (`rivalcfg/devices/prime_mini.py`'s `input_range: [50, 18000, 50]`),
/// linear formula `byte_value = dpi / 50` (`output_range: [0x01, 0x0168,
/// 1]`), expanded to a full lookup table — computed directly from the
/// verified formula/ranges (not guessed) — so the Prime Mini's DPI encoding
/// uses the exact same lookup-table code path
/// ([`crate::device::profile::DpiProfile`]/`set_dpi`) as every other
/// device, with `value_width: 2` the only thing that differs (its raw value
/// runs up to `0x0168`, needing both bytes; see
/// [`crate::device::profile::DpiProfile::value_width`]).
pub const PRIME_MINI_DPI_MAPPING: &[(u16, u16)] = &[
    (50, 0x01), (100, 0x02), (150, 0x03), (200, 0x04),
    (250, 0x05), (300, 0x06), (350, 0x07), (400, 0x08),
    (450, 0x09), (500, 0x0A), (550, 0x0B), (600, 0x0C),
    (650, 0x0D), (700, 0x0E), (750, 0x0F), (800, 0x10),
    (850, 0x11), (900, 0x12), (950, 0x13), (1000, 0x14),
    (1050, 0x15), (1100, 0x16), (1150, 0x17), (1200, 0x18),
    (1250, 0x19), (1300, 0x1A), (1350, 0x1B), (1400, 0x1C),
    (1450, 0x1D), (1500, 0x1E), (1550, 0x1F), (1600, 0x20),
    (1650, 0x21), (1700, 0x22), (1750, 0x23), (1800, 0x24),
    (1850, 0x25), (1900, 0x26), (1950, 0x27), (2000, 0x28),
    (2050, 0x29), (2100, 0x2A), (2150, 0x2B), (2200, 0x2C),
    (2250, 0x2D), (2300, 0x2E), (2350, 0x2F), (2400, 0x30),
    (2450, 0x31), (2500, 0x32), (2550, 0x33), (2600, 0x34),
    (2650, 0x35), (2700, 0x36), (2750, 0x37), (2800, 0x38),
    (2850, 0x39), (2900, 0x3A), (2950, 0x3B), (3000, 0x3C),
    (3050, 0x3D), (3100, 0x3E), (3150, 0x3F), (3200, 0x40),
    (3250, 0x41), (3300, 0x42), (3350, 0x43), (3400, 0x44),
    (3450, 0x45), (3500, 0x46), (3550, 0x47), (3600, 0x48),
    (3650, 0x49), (3700, 0x4A), (3750, 0x4B), (3800, 0x4C),
    (3850, 0x4D), (3900, 0x4E), (3950, 0x4F), (4000, 0x50),
    (4050, 0x51), (4100, 0x52), (4150, 0x53), (4200, 0x54),
    (4250, 0x55), (4300, 0x56), (4350, 0x57), (4400, 0x58),
    (4450, 0x59), (4500, 0x5A), (4550, 0x5B), (4600, 0x5C),
    (4650, 0x5D), (4700, 0x5E), (4750, 0x5F), (4800, 0x60),
    (4850, 0x61), (4900, 0x62), (4950, 0x63), (5000, 0x64),
    (5050, 0x65), (5100, 0x66), (5150, 0x67), (5200, 0x68),
    (5250, 0x69), (5300, 0x6A), (5350, 0x6B), (5400, 0x6C),
    (5450, 0x6D), (5500, 0x6E), (5550, 0x6F), (5600, 0x70),
    (5650, 0x71), (5700, 0x72), (5750, 0x73), (5800, 0x74),
    (5850, 0x75), (5900, 0x76), (5950, 0x77), (6000, 0x78),
    (6050, 0x79), (6100, 0x7A), (6150, 0x7B), (6200, 0x7C),
    (6250, 0x7D), (6300, 0x7E), (6350, 0x7F), (6400, 0x80),
    (6450, 0x81), (6500, 0x82), (6550, 0x83), (6600, 0x84),
    (6650, 0x85), (6700, 0x86), (6750, 0x87), (6800, 0x88),
    (6850, 0x89), (6900, 0x8A), (6950, 0x8B), (7000, 0x8C),
    (7050, 0x8D), (7100, 0x8E), (7150, 0x8F), (7200, 0x90),
    (7250, 0x91), (7300, 0x92), (7350, 0x93), (7400, 0x94),
    (7450, 0x95), (7500, 0x96), (7550, 0x97), (7600, 0x98),
    (7650, 0x99), (7700, 0x9A), (7750, 0x9B), (7800, 0x9C),
    (7850, 0x9D), (7900, 0x9E), (7950, 0x9F), (8000, 0xA0),
    (8050, 0xA1), (8100, 0xA2), (8150, 0xA3), (8200, 0xA4),
    (8250, 0xA5), (8300, 0xA6), (8350, 0xA7), (8400, 0xA8),
    (8450, 0xA9), (8500, 0xAA), (8550, 0xAB), (8600, 0xAC),
    (8650, 0xAD), (8700, 0xAE), (8750, 0xAF), (8800, 0xB0),
    (8850, 0xB1), (8900, 0xB2), (8950, 0xB3), (9000, 0xB4),
    (9050, 0xB5), (9100, 0xB6), (9150, 0xB7), (9200, 0xB8),
    (9250, 0xB9), (9300, 0xBA), (9350, 0xBB), (9400, 0xBC),
    (9450, 0xBD), (9500, 0xBE), (9550, 0xBF), (9600, 0xC0),
    (9650, 0xC1), (9700, 0xC2), (9750, 0xC3), (9800, 0xC4),
    (9850, 0xC5), (9900, 0xC6), (9950, 0xC7), (10000, 0xC8),
    (10050, 0xC9), (10100, 0xCA), (10150, 0xCB), (10200, 0xCC),
    (10250, 0xCD), (10300, 0xCE), (10350, 0xCF), (10400, 0xD0),
    (10450, 0xD1), (10500, 0xD2), (10550, 0xD3), (10600, 0xD4),
    (10650, 0xD5), (10700, 0xD6), (10750, 0xD7), (10800, 0xD8),
    (10850, 0xD9), (10900, 0xDA), (10950, 0xDB), (11000, 0xDC),
    (11050, 0xDD), (11100, 0xDE), (11150, 0xDF), (11200, 0xE0),
    (11250, 0xE1), (11300, 0xE2), (11350, 0xE3), (11400, 0xE4),
    (11450, 0xE5), (11500, 0xE6), (11550, 0xE7), (11600, 0xE8),
    (11650, 0xE9), (11700, 0xEA), (11750, 0xEB), (11800, 0xEC),
    (11850, 0xED), (11900, 0xEE), (11950, 0xEF), (12000, 0xF0),
    (12050, 0xF1), (12100, 0xF2), (12150, 0xF3), (12200, 0xF4),
    (12250, 0xF5), (12300, 0xF6), (12350, 0xF7), (12400, 0xF8),
    (12450, 0xF9), (12500, 0xFA), (12550, 0xFB), (12600, 0xFC),
    (12650, 0xFD), (12700, 0xFE), (12750, 0xFF), (12800, 0x100),
    (12850, 0x101), (12900, 0x102), (12950, 0x103), (13000, 0x104),
    (13050, 0x105), (13100, 0x106), (13150, 0x107), (13200, 0x108),
    (13250, 0x109), (13300, 0x10A), (13350, 0x10B), (13400, 0x10C),
    (13450, 0x10D), (13500, 0x10E), (13550, 0x10F), (13600, 0x110),
    (13650, 0x111), (13700, 0x112), (13750, 0x113), (13800, 0x114),
    (13850, 0x115), (13900, 0x116), (13950, 0x117), (14000, 0x118),
    (14050, 0x119), (14100, 0x11A), (14150, 0x11B), (14200, 0x11C),
    (14250, 0x11D), (14300, 0x11E), (14350, 0x11F), (14400, 0x120),
    (14450, 0x121), (14500, 0x122), (14550, 0x123), (14600, 0x124),
    (14650, 0x125), (14700, 0x126), (14750, 0x127), (14800, 0x128),
    (14850, 0x129), (14900, 0x12A), (14950, 0x12B), (15000, 0x12C),
    (15050, 0x12D), (15100, 0x12E), (15150, 0x12F), (15200, 0x130),
    (15250, 0x131), (15300, 0x132), (15350, 0x133), (15400, 0x134),
    (15450, 0x135), (15500, 0x136), (15550, 0x137), (15600, 0x138),
    (15650, 0x139), (15700, 0x13A), (15750, 0x13B), (15800, 0x13C),
    (15850, 0x13D), (15900, 0x13E), (15950, 0x13F), (16000, 0x140),
    (16050, 0x141), (16100, 0x142), (16150, 0x143), (16200, 0x144),
    (16250, 0x145), (16300, 0x146), (16350, 0x147), (16400, 0x148),
    (16450, 0x149), (16500, 0x14A), (16550, 0x14B), (16600, 0x14C),
    (16650, 0x14D), (16700, 0x14E), (16750, 0x14F), (16800, 0x150),
    (16850, 0x151), (16900, 0x152), (16950, 0x153), (17000, 0x154),
    (17050, 0x155), (17100, 0x156), (17150, 0x157), (17200, 0x158),
    (17250, 0x159), (17300, 0x15A), (17350, 0x15B), (17400, 0x15C),
    (17450, 0x15D), (17500, 0x15E), (17550, 0x15F), (17600, 0x160),
    (17650, 0x161), (17700, 0x162), (17750, 0x163), (17800, 0x164),
    (17850, 0x165), (17900, 0x166), (17950, 0x167), (18000, 0x168),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truemove_core_mapping_has_84_entries_spanning_200_to_8500() {
        assert_eq!(TRUEMOVE_CORE_MAPPING.len(), 84);
        assert_eq!(TRUEMOVE_CORE_MAPPING.first(), Some(&(200, 0x04)));
        assert_eq!(TRUEMOVE_CORE_MAPPING.last(), Some(&(8500, 0xC5)));
    }

    #[test]
    fn truemove_air_mapping_has_180_entries_spanning_100_to_18000() {
        assert_eq!(TRUEMOVE_AIR_MAPPING.len(), 180);
        assert_eq!(TRUEMOVE_AIR_MAPPING.first(), Some(&(100, 0x00)));
        assert_eq!(TRUEMOVE_AIR_MAPPING.last(), Some(&(18000, 0xD6)));
    }

    #[test]
    fn prime_mini_mapping_has_360_entries_spanning_50_to_18000() {
        assert_eq!(PRIME_MINI_DPI_MAPPING.len(), 360);
        assert_eq!(PRIME_MINI_DPI_MAPPING.first(), Some(&(50, 0x01)));
        assert_eq!(PRIME_MINI_DPI_MAPPING.last(), Some(&(18000, 0x168)));
    }

    /// Every entry must match rivalcfg's `byte_value = dpi / 50` formula —
    /// guards against a transcription slip anywhere in the 360-entry table.
    #[test]
    fn prime_mini_mapping_matches_its_own_formula() {
        for &(dpi, value) in PRIME_MINI_DPI_MAPPING {
            assert_eq!(value, dpi / 50, "dpi {} encoded as 0x{:x}, expected {}", dpi, value, dpi / 50);
        }
    }
}
