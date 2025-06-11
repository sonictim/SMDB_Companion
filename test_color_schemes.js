// Quick test to verify the Native Instruments scheme is in all constants
import { 
  defaultSchemeNames, 
  defaultColorSchemes, 
  themeOptions, 
  colorThemes 
} from './src/stores/colors.ts';

console.log('=== TESTING COLOR SCHEME CONSOLIDATION ===\n');

console.log('1. defaultSchemeNames:');
console.log(defaultSchemeNames);
console.log('✓ Contains Native Instruments:', defaultSchemeNames.includes('Native Instruments'));

console.log('\n2. defaultColorSchemes:');
defaultColorSchemes.forEach(scheme => console.log(`  - ${scheme.name}`));
console.log('✓ Contains Native Instruments:', defaultColorSchemes.some(s => s.name === 'Native Instruments'));

console.log('\n3. themeOptions:');
themeOptions.forEach(option => console.log(`  - ${option.value}: ${option.label}`));
console.log('✓ Contains native key:', themeOptions.some(o => o.value === 'native'));

console.log('\n4. colorThemes:');
console.log('Available theme keys:', Object.keys(colorThemes));
console.log('✓ Contains native key:', 'native' in colorThemes);

console.log('\n5. Native theme colors preview:');
if (colorThemes.native) {
  console.log('Primary BG:', colorThemes.native.primaryBg);
  console.log('Accent Color:', colorThemes.native.accentColor);
  console.log('Topbar Color:', colorThemes.native.topbarColor);
} else {
  console.log('❌ Native theme not found!');
}

console.log('\n=== ALL CONSTANTS GENERATED FROM SINGLE SOURCE ===');
