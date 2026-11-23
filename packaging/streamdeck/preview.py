"""Draws every key in each look with Qt's own SVG renderer (the one Stream
Deck uses) onto one sheet: resting, on, not in a call, HaloCord unreachable.
Run: python preview.py <node.exe> <out.png>"""
import json
import os
import subprocess
import sys

from PySide6.QtCore import QByteArray, QRectF
from PySide6.QtGui import QColor, QFont, QGuiApplication, QImage, QPainter
from PySide6.QtSvg import QSvgRenderer

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = r"""
const {ACTIONS,RESTING,unreachable}=require('./com.davidhifi.yapfast.sdPlugin/bin/actions.js');
const {key}=require('./com.davidhifi.yapfast.sdPlugin/bin/icons.js');
const away={...RESTING,inCall:false};
console.log(JSON.stringify(ACTIONS.map(a=>({name:a.name,keys:[
  a.look({...RESTING,...(a.off||{})}),a.look({...RESTING,...(a.on||{})}),a.look(away),unreachable(a)].map(key)}))));
"""

node, out = sys.argv[1], sys.argv[2]
rows = json.loads(subprocess.run([node, "-e", SCRIPT], cwd=HERE, capture_output=True, text=True,
                                 check=True, creationflags=subprocess.CREATE_NO_WINDOW).stdout)
app = QGuiApplication([])
cell, pad, label_w = 144, 10, 150
sheet = QImage(label_w + 4 * (cell + pad) + pad, len(rows) * (cell + pad) + pad + 30, QImage.Format_ARGB32)
sheet.fill(QColor("#11111B"))
paint = QPainter(sheet)
paint.setRenderHint(QPainter.Antialiasing)
paint.setPen(QColor("#A6ADC8"))
paint.setFont(QFont("Segoe UI", 10))
for x, head in enumerate(["resting", "on", "no call", "unreachable"]):
    paint.drawText(label_w + pad + x * (cell + pad), 20, head)
for y, row in enumerate(rows):
    top = 30 + pad + y * (cell + pad)
    paint.drawText(10, top + cell // 2, row["name"])
    for x, svg in enumerate(row["keys"]):
        renderer = QSvgRenderer(QByteArray(svg.encode("utf-8")))
        renderer.render(paint, QRectF(label_w + pad + x * (cell + pad), top, cell, cell))
paint.end()
sheet.save(out)
print(out)
