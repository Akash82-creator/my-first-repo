/**
 * Unit Converter - Convert between various units
 */

class UnitConverter {
    static conversions = {
        length: {
            base: 'm',
            units: {
                m: 1,
                km: 1000,
                cm: 0.01,
                mm: 0.001,
                um: 1e-6,
                nm: 1e-9,
                mile: 1609.344,
                yard: 0.9144,
                foot: 0.3048,
                inch: 0.0254,
                nautical_mile: 1852,
                light_year: 9.461e15,
                au: 1.496e11
            }
        },
        area: {
            base: 'm²',
            units: {
                m²: 1,
                km²: 1e6,
                cm²: 1e-4,
                mm²: 1e-6,
                hectare: 10000,
                acre: 4046.86,
                sq_mile: 2.59e6,
                sq_yard: 0.836127,
                sq_foot: 0.092903,
                sq_inch: 6.4516e-4
            }
        },
        volume: {
            base: 'm³',
            units: {
                m³: 1,
                liter: 0.001,
                ml: 1e-6,
                gallon_us: 0.00378541,
                quart_us: 0.000946353,
                pint_us: 0.000473176,
                cup_us: 0.000236588,
                fluid_oz_us: 2.95735e-5,
                tbsp: 1.47868e-5,
                tsp: 4.92892e-6,
                cubic_foot: 0.0283168,
                cubic_inch: 1.63871e-5
            }
        },
        mass: {
            base: 'kg',
            units: {
                kg: 1,
                g: 0.001,
                mg: 1e-6,
                ug: 1e-9,
                tonne: 1000,
                lb: 0.453592,
                oz: 0.0283495,
                stone: 6.35029,
                ton_us: 907.185,
                ton_uk: 1016.05,
                carat: 0.0002,
                grain: 6.47989e-5
            }
        },
        temperature: {
            special: true,
            convert: (value, from, to) => {
                // Convert to Celsius first
                let celsius;
                switch (from) {
                    case 'C': celsius = value; break;
                    case 'F': celsius = (value - 32) * 5/9; break;
                    case 'K': celsius = value - 273.15; break;
                    case 'R': celsius = (value - 491.67) * 5/9; break;
                    default: celsius = value;
                }
                // Convert from Celsius to target
                switch (to) {
                    case 'C': return celsius;
                    case 'F': return celsius * 9/5 + 32;
                    case 'K': return celsius + 273.15;
                    case 'R': return (celsius + 273.15) * 9/5;
                    default: return celsius;
                }
            }
        },
        pressure: {
            base: 'Pa',
            units: {
                Pa: 1,
                kPa: 1000,
                MPa: 1e6,
                bar: 1e5,
                atm: 101325,
                psi: 6894.76,
                torr: 133.322,
                mmHg: 133.322,
                inHg: 3386.39
            }
        },
        energy: {
            base: 'J',
            units: {
                J: 1,
                kJ: 1000,
                MJ: 1e6,
                cal: 4.184,
                kcal: 4184,
                Wh: 3600,
                kWh: 3.6e6,
                eV: 1.60218e-19,
                BTU: 1055.06,
                therm: 1.055e8,
                ft_lb: 1.35582
            }
        },
        power: {
            base: 'W',
            units: {
                W: 1,
                kW: 1000,
                MW: 1e6,
                GW: 1e9,
                hp: 745.7,
                hp_metric: 735.5,
                BTU_h: 0.293071,
                cal_s: 4.184
            }
        },
        time: {
            base: 's',
            units: {
                s: 1,
                ms: 0.001,
                us: 1e-6,
                ns: 1e-9,
                minute: 60,
                hour: 3600,
                day: 86400,
                week: 604800,
                month: 2.628e6,
                year: 3.154e7
            }
        },
        speed: {
            base: 'm/s',
            units: {
                'm/s': 1,
                'km/h': 0.277778,
                'mph': 0.44704,
                knot: 0.514444,
                'ft/s': 0.3048,
                mach: 343,
                c: 299792458
            }
        },
        data: {
            base: 'B',
            units: {
                B: 1,
                KB: 1024,
                MB: 1048576,
                GB: 1073741824,
                TB: 1099511627776,
                PB: 1125899906842624,
                Kb: 128,
                Mb: 131072,
                Gb: 134217728,
                Tb: 137438953472
            }
        }
    };

    static getUnits(category) {
        const cat = this.conversions[category];
        if (!cat) return [];
        if (cat.special) {
            return Object.keys(cat.units || {}).length > 0 ? Object.keys(cat.units) : ['C', 'F', 'K', 'R'];
        }
        return Object.keys(cat.units);
    }

    static convert(value, from, to, category) {
        const cat = this.conversions[category];
        if (!cat) throw new Error(`Unknown category: ${category}`);
        
        if (cat.special && cat.convert) {
            return cat.convert(value, from, to);
        }
        
        if (!cat.units[from] || !cat.units[to]) {
            throw new Error('Invalid unit');
        }
        
        // Convert to base, then to target
        const baseValue = value * cat.units[from];
        return baseValue / cat.units[to];
    }

    static formatResult(value, unit) {
        if (Math.abs(value) < 1e-6 || Math.abs(value) >= 1e9) {
            return `${value.toExponential(6)} ${unit}`;
        }
        return `${value.toFixed(6).replace(/\.?0+$/, '')} ${unit}`;
    }
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = UnitConverter;
}
