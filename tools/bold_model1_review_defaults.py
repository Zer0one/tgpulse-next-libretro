#!/usr/bin/env python3
"""Preserve native XLSX rich-text defaults after Artifact Tool export.

The bundled Artifact Tool API has no documented rich-text authoring operation.
This narrow OOXML pass adds bold runs only in Observed Values; native tables,
styles, filters and other ZIP members remain unchanged. Re-running is safe.
"""
import argparse
import io
import json
from pathlib import Path
import subprocess
import xml.etree.ElementTree as ET
import zipfile
from generate_model1_nvram import ROOT, display_label, workbook_rows, workbook_records

URI = 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'
NS = {'m': URI}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('workbook', type=Path)
    args = parser.parse_args()
    rows = workbook_rows(args.workbook)
    records = workbook_records(rows, 'Core options')
    headers = rows['Core options'][0]
    assert 'Native Default' not in headers.values()
    observed_column = next(c for c, h in headers.items() if h == 'Observed Values')
    sets = sorted({r['set'] for r in records})
    docs = json.loads(subprocess.check_output(['ruby', '-ryaml', '-rjson', '-e',
        'puts JSON.generate(ARGV.map{|p| YAML.safe_load(File.read(p), permitted_classes: [], aliases: true)})',
        *[str(ROOT/f'data/diagnostic-menus/{s}.yaml') for s in sets]], text=True))
    defaults = {(doc['game']['set'], option['label'].casefold()): display_label(option['native_default'])
        for doc in docs for menu in doc['menus'].values() for option in menu.get('options', [])}
    with zipfile.ZipFile(args.workbook) as source:
        parts = {name: source.read(name) for name in source.namelist()}
    # This workbook's first sheet is the primary review; preserve all namespace prefixes.
    name = 'xl/worksheets/sheet1.xml'
    for _, (prefix, uri) in ET.iterparse(io.BytesIO(parts[name]), events=['start-ns']):
        ET.register_namespace(prefix, uri)
    sheet = ET.fromstring(parts[name])
    cells = {c.get('r'): c for c in sheet.findall('.//m:sheetData/m:row/m:c', NS)}
    for index, row in enumerate(records, 2):
        value = row['observed values']
        default = defaults[row['set'], row['setting'].casefold()]
        values = value.split(', ')
        assert values.count(default) == 1, (row['set'], row['setting'], default, values)
        cell = cells[f'{observed_column}{index}']
        for child in list(cell): cell.remove(child)
        cell.set('t', 'inlineStr')
        rich = ET.SubElement(cell, f'{{{URI}}}is')
        for i, item in enumerate(values):
            if i:
                run = ET.SubElement(rich, f'{{{URI}}}r')
                text = ET.SubElement(run, f'{{{URI}}}t'); text.text = ', '
                text.set('{http://www.w3.org/XML/1998/namespace}space','preserve')
            run = ET.SubElement(rich, f'{{{URI}}}r')
            if item == default:
                properties = ET.SubElement(run, f'{{{URI}}}rPr')
                ET.SubElement(properties, f'{{{URI}}}b')
            ET.SubElement(run, f'{{{URI}}}t').text = item
        bold = [r.find('m:t', NS).text for r in rich.findall('m:r', NS) if r.find('m:rPr/m:b',NS) is not None]
        assert bold == [default]
    parts[name] = ET.tostring(sheet, encoding='utf-8', xml_declaration=True)
    temporary = args.workbook.with_suffix('.rich.tmp')
    with zipfile.ZipFile(temporary, 'w', zipfile.ZIP_DEFLATED) as target:
        for member, data in parts.items(): target.writestr(member, data)
    temporary.replace(args.workbook)
    # No values change, only presentation.
    assert workbook_rows(args.workbook) == rows
    print(json.dumps({'default_values_bold': len(records), 'separate_default_column': False}))

if __name__ == '__main__': main()
