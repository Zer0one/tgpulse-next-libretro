// Edit the existing review workbook using the bundled Artifact Tool runtime.
// JSON input preserves sheet order and source fields; a separate default column is omitted.
import fs from 'node:fs/promises';
import {createRequire} from 'node:module';
const require = createRequire(import.meta.url);
const runtime = process.env.CODEX_WORKSPACE_NODE_MODULES;
if (!runtime) throw new Error('Set CODEX_WORKSPACE_NODE_MODULES from load_workspace_dependencies');
const {FileBlob, SpreadsheetFile} = await import(require.resolve('@oai/artifact-tool', {paths:[runtime]}));
const [input, changes, output, previews] = process.argv.slice(2);
const rows = JSON.parse(await fs.readFile(changes, 'utf8'));
const wb = await SpreadsheetFile.importXlsx(await FileBlob.load(input));
const definitions = [
 ['Core options', 'ABCDEFGHIJKL', 'CoreOptionsReview', 110],
 ['Campaign', 'ABCDEFGHIJ', 'Model1Campaign', 10],
 ['Calibration', 'ABCDEFGH', 'Model1Calibration', 4],
 ['Automatic setup', 'ABCDEFGH', 'Model1AutomaticSetup', 10],
];
for (const [name, letters, tableName, count] of definitions) {
 const sheet = wb.worksheets.getItem(name);
 for (const table of [...sheet.tables.items]) table.delete();
 let columns = [...letters];
 if (name === 'Core options') columns = Object.keys(rows[name][0]).filter(c => rows[name][0][c] && rows[name][0][c].toLowerCase() !== 'native default');
 const matrix = rows[name].map(row => columns.map(col => row[col] || null));
 const last = String.fromCharCode(64 + columns.length);
 // The main table loses its separate default column; other source cells stay in place.
 const old = sheet.getRange(name === 'Core options' ? 'A1:L111' : `A1:${last}${matrix.length}`);
 old.clear({applyTo:'all'});
 const range = sheet.getRange(`A1:${last}${matrix.length}`);
 range.values = matrix;
 range.format.font = {name:'Arial',size:11,color:'#182B22'};
 range.format.wrapText = true;
 range.format.verticalAlignment = 'center';
 range.format.columnWidth = 24;
 const table = sheet.tables.add(`A1:${last}${count+1}`,true,tableName);
 table.style = 'TableStyleMedium8';
 table.showFilterButton = true;
 sheet.showGridLines = false;
 sheet.freezePanes.freezeRows(1);
 sheet.freezePanes.freezeColumns(name === 'Core options' ? 2 : 1);
 const widths = name === 'Core options' ? [24,16,38,70,24,48,42,56,34,48,24] :
  name === 'Campaign' ? [16,26,16,16,22,22,26,34,44,64] :
  name === 'Calibration' ? [20,26,36,38,38,42,54,54] : [18,28,22,20,64,38,24,58];
 widths.forEach((width,index) => sheet.getRange(`${String.fromCharCode(65+index)}1`).format.columnWidth=width);
 range.format.autofitRows();
 sheet.getRange(`A1:${last}1`).format.rowHeight=38;
 sheet.getRange(`A1:${last}1`).format.font={name:'Arial',size:11,bold:true,color:'#FFFFFF'};
 // Explicit reference colours keep previews and native Excel consistent even
 // when an imported workbook's theme maps standard table accents to grayscale.
 range.format.fill='#FFFFFF';
 for(let row=2;row<=count+1;row+=2) sheet.getRange(`A${row}:${last}${row}`).format.fill='#E2EFDA';
 sheet.getRange(`A1:${last}1`).format.fill='#548235';
 // Preserve review cues using conditional formatting within the filterable table.
 if (name === 'Core options') {
  sheet.getRange('E2:E111').conditionalFormats.add('containsText',{text:'Yes',format:{fill:'#D9EAD3',font:{bold:true}}});
  sheet.getRange('K2:K111').dataValidation={rule:{type:'list',values:['Approved','Not selected']}};
 }
}
wb.recalculate();
console.log((await wb.inspect({kind:'region',sheetId:'Core options',range:'A1:E4',maxChars:1600,tableMaxRows:4,tableMaxCols:5})).ndjson);
await fs.mkdir(previews,{recursive:true});
for (const [name] of definitions) {
 const blob=await wb.render({sheetName:name,range:name==='Core options'?'A1:E5':'A1:F4',scale:1,format:'png'});
 await fs.writeFile(`${previews}/${name}.png`,new Uint8Array(await blob.arrayBuffer()));
}
const xlsx=await SpreadsheetFile.exportXlsx(wb);await xlsx.save(output);
console.log('Saved filterable review tables:', output);
