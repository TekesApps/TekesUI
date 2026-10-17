#!/usr/bin/env python3
"""Deterministic original SVG proposals. Does not read or overwrite legacy artwork."""
from pathlib import Path
import json, math, xml.etree.ElementTree as ET
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'svg'
OUT.mkdir(exist_ok=True)
items=[]
def path(d): return f'<path d="{d}"/>'
def circle(x,y,r,fill=False): return f'<circle cx="{x}" cy="{y}" r="{r}"'+(' fill="currentColor" stroke="none"' if fill else '')+'/>'
def rect(x,y,w,h,rx=2): return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}"/>'
def add(name,label,group,usage,body,kind='new',reason=''):
    svg=f'<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">{body}</svg>\n'
    ET.fromstring(svg)
    (OUT/f'tekes-{name}.svg').write_text(svg)
    items.append(dict(name=f'tekes-{name}',label=label,group=group,usage=usage,kind=kind,reason=reason))
p=path;c=circle;r=rect
# Shared gaps already visible in the current IconRole/SF fallback vocabulary.
add('zoom-in','放大','shared','三应用的内容缩放',c(10,10,6.5)+p('M15 15l5.5 5.5 M7 10h6 M10 7v6'))
add('zoom-out','缩小','shared','三应用的内容缩放',c(10,10,6.5)+p('M15 15l5.5 5.5 M7 10h6'))
add('open-external','外部打开','shared','系统应用或外部链接',p('M13 4h7v7 M20 4l-9 9 M9 5H6a2 2 0 0 0-2 2v11a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2v-3'))
add('attach','添加附件','shared','文件和素材附加',p('M8 13.5l6.7-6.7a2.8 2.8 0 0 1 4 4l-8.3 8.3a4.5 4.5 0 0 1-6.4-6.4l8.4-8.4 M8 13.5l6.5-6.5'))
add('filter','筛选','shared','列表筛选',p('M3.5 5h17l-6.5 7.5v6l-4 2v-8L3.5 5Z'))
add('sort','排序','shared','名称、时间或顺序排序',p('M6 4v16 M3.5 17.5 6 20l2.5-2.5 M12 6h8 M12 12h6 M12 18h3'))
add('undo','撤销','shared','文档编辑撤销',p('M8 5 3.5 9.5 8 14 M4 9.5h10a5.5 5.5 0 0 1 0 11h-3'))
add('redo','重做','shared','文档编辑重做',p('M16 5l4.5 4.5L16 14 M20 9.5H10a5.5 5.5 0 0 0 0 11h3'))
add('diff-annotation','差异批注','shared','差异审核和批注入口',p('M4 4h12a2 2 0 0 1 2 2v5 M4 4v15h6 M7 8h7 M7 12h3 M13 14h8v6h-4l-4 1v-7Z'))
add('file-link','关联文件','shared','关联已有文件',p('M13 3H5v18h7 M13 3l5 5h-5V3 M15 14l1.5-1.5a3 3 0 0 1 4.2 4.2L19 18.5 M17 17l-1.5 1.5a3 3 0 0 1-4.2-4.2L13 12.5'))
# CAD: make the domain actions explicit instead of reusing generic files/settings.
add('cad-layers','图层','cad','显示、隐藏和组织图层',p('M3 7.5 12 3l9 4.5-9 4.5-9-4.5Z M3 12l9 4.5 9-4.5 M3 16.5l9 4.5 9-4.5'))
add('cad-dimension','尺寸标注','cad','工程尺寸及人工审核',p('M4 5v14 M20 5v14 M4 12h16 M7 9l-3 3 3 3 M17 9l3 3-3 3'))
add('cad-snap','对象捕捉','cad','端点、中点等几何捕捉',p('M3 8V3h5 M16 3h5v5 M21 16v5h-5 M8 21H3v-5')+r(9,9,6,6,0))
add('cad-window-select','窗口选择','cad','左到右：完全包围实体',r(3.5,3.5,17,17,0)+p('M7 15 10 8l7 7H7Z'))
add('cad-crossing-select','交叉选择','cad','右到左：框内及穿越实体',p('M3.5 7V3.5H7 M11 3.5h3 M18 3.5h2.5V7 M20.5 11v3 M20.5 18v2.5H18 M14 20.5h-3 M7 20.5H3.5V18 M3.5 14v-3 M7 15l8-7 6 7'))
add('cad-line','直线','cad','直线实体工具',p('M5 19 19 5')+r(3,17,4,4,0)+r(17,3,4,4,0))
add('cad-polyline','多段线','cad','连接顶点的多段线',p('M4.5 18 10 6l9 10')+r(3,16.5,3,3,0)+r(8.5,4.5,3,3,0)+r(17.5,14.5,3,3,0))
add('cad-arc','圆弧','cad','圆弧实体工具',p('M5 18A13 13 0 0 1 18 5')+c(5,18,1.7)+c(18,5,1.7))
add('cad-block','块','cad','块定义和实例',p('M12 3 21 8v9l-9 4-9-4V8l9-5Z M3 8l9 4 9-4 M12 12v9'))
add('cad-layout','图纸布局','cad','模型空间、图纸及视口',r(3,4,18,16,1)+r(6,7,9,7,0)+p('M17 15h2 M15 18h4'))
add('cad-pdf-import','PDF 导入','cad','PDF 转换及来源关联',p('M11 3H4v18h10 M11 3l5 5h-5V3 M7 12h4 M7 16h2 M19 10v10 M16 17l3 3 3-3'))
add('cad-export','CAD 导出','cad','保存可交换绘图产物',p('M11 3H4v18h12v-7 M11 3l5 5h-5V3 M14 11h7 M18 8l3 3-3 3'))
add('cad-compare','图形对比','cad','来源和转换结果几何对照',p('M12 3v18 M8 6H3v12h5 M16 6h5v12h-5 M5 15l3-6 M16 15l3-6'))
add('cad-table-extract','提取表格','cad','PDF 表格附件和 CSV 导出',r(3,4,18,16,1)+p('M3 9h18 M8 9v11 M8 14h13'))
# Drama: the ten current workspaces + likely object-level editing actions.
add('drama-brief','创作简报','drama','Brief 工作区',r(5,3,14,18,1.5)+p('M8 7h8 M8 11h8 M8 15h5'))
add('drama-character','角色','drama','Characters 工作区及人物对象',c(12,7,3)+p('M5 21v-2a7 7 0 0 1 14 0v2 M9 14l3 3 3-3'))
add('drama-script','剧本','drama','Script 工作区',p('M5 3h13v14H8a2 2 0 0 0 0 4H5V3Z M8 21h11v-4 M8 7h7 M8 11h5'))
add('drama-directing','导演','drama','Directing 工作区',p('M4 8 19 3l1.5 4-15 5L4 8Z M8 6.7l2.8 3.1 M14 4.7l2.8 3.1 M4 12h16v9H4v-9Z'))
add('drama-production','制作','drama','Production 工作区与拍摄组织',r(3,7,18,14,2)+p('M8 7V4h8v3 M3 12h18 M10 11v3h4v-3'))
add('drama-editing','剪辑','drama','Editing 工作区',p('M3 5h18v14H3V5Z M7 5v14 M17 5v14 M3 10h4 M3 14h4 M17 10h4 M17 14h4 M10 9l4 3-4 3V9Z'))
add('drama-delivery','交付','drama','Delivery 工作区',p('M12 3 21 7.5v10L12 22 3 17.5v-10L12 3Z M3 7.5l9 4.5 9-4.5 M12 12v10 M8 14l2 2 5-5'))
add('drama-audience','观众实验','drama','Audience Lab 工作区',c(8,7,2.7)+p('M3 20v-2a5 5 0 0 1 10 0v2 M16 5a3 3 0 0 1 0 6 M17 14a5 5 0 0 1 4.5 5v1'))
add('drama-references','参考资料','drama','References 工作区',r(4,6,17,15,1.5)+p('M3 17V3H17 M7 16l4-4 3 3 2-2 3 5')+c(16,10,1.2))
add('drama-collaboration','协作','drama','Collaboration 工作区',p('M3 4h13v9h-6l-4 3v-3H3V4Z M19 8h2v11h-4l-3 3v-3h-3v-3'))
add('drama-shot','镜头','drama','分镜与镜头清单对象',r(3,6,13,13,2)+p('M16 10l5-3v11l-5-3 M6 3h7'))
add('drama-storyboard','分镜板','drama','镜头图板及顺序',r(3,4,18,16,1.5)+p('M3 12h18 M12 4v16 M6 9l3-2 M15 16h3'))
add('drama-audio','声音轨','drama','对白、音效与配乐轨',p('M3 10v4 M7.5 6v12 M12 3v18 M16.5 7v10 M21 10v4'))
add('drama-subtitles','字幕','drama','字幕及对白时间轴',r(3,5,18,14,2)+p('M6 11h5 M14 11h4 M6 15h3 M12 15h6'))
# Six equal teeth, with a clear hub; radial construction is independent of system assets.
gear_points=[]
for tooth in range(6):
    for degrees, radius in [(-10,9), (10,9), (20,7), (40,7)]:
        angle=math.radians(tooth*60 + degrees - 90)
        gear_points.append((12+radius*math.cos(angle),12+radius*math.sin(angle)))
gear_path='M'+' L'.join(f'{x:.3f} {y:.3f}' for x,y in gear_points)+' Z'
# Existing symbols selected for concrete semantic or small-size defects.
add('settings','设置','refine','三应用统一设置入口',p(gear_path)+c(12,12,3),'refine','旧稿同心圆加十字，容易被读为准星；改为明确的齿轮轮廓')
add('new-goal','新增目标','refine','创建目标',c(10,11,7)+c(10,11,3)+p('M18 14v7 M14 17.5h7'),'refine','旧稿没有显著的加号；将新增标记移到右下，主轮廓留出负空间')
add('goal-completed','完成目标','refine','目标完成状态',c(12,12,8.5)+p('M7.5 12l3 3 6-6'),'refine','去掉挤在内环中的小勾，让完成标记在12pt仍有明确轮廓')
add('memory','记忆','refine','长期记忆入口',r(6,6,12,12,2)+r(10,10,4,4,0.7)+p('M9 3v3 M15 3v3 M9 18v3 M15 18v3 M3 9h3 M3 15h3 M18 9h3 M18 15h3'),'refine','减少密集引脚并扩大内部负空间，保留芯片语义')
add('think','思考','refine','模型思考状态',p('M7 16a5 5 0 0 1-2-9.5 6 6 0 0 1 11.5-.5A5 5 0 0 1 17 16H7Z')+c(8,19,1.2,True)+c(5,21,0.8,True),'refine','降低下垂尾部的视觉重心，减小底部小点对边界的挤压')
add('router','路由','refine','模型或任务路由',p('M3 12h6 M9 12l6-6h6 M9 12l6 6h6 M18 3l3 3-3 3 M18 15l3 3-3 3'),'refine','从闭合三角节点改为一入两出的方向路径，避免读成分享')
add('provider','服务商','refine','Provider 设置与服务入口',r(4,3,16,8,2)+r(4,13,16,8,2)+c(7.5,7,0.9,True)+c(7.5,17,0.9,True)+p('M12 7h5 M12 17h5'),'refine','旧叠层形与CAD图层冲突；改用服务器抽屉，与图层明确区分')
add('appearance','外观','refine','浅色、深色和外观设置',c(12,12,8.5)+'<path d="M12 3.5a8.5 8.5 0 0 1 0 17V3.5Z" fill="currentColor" stroke="none"/>','refine','调色盘多圆点在小尺寸过密；用明确的明暗分区表达外观，而非绘图工具')
# Retired five are drawn independently; do not load, inspect or trace their originals.
add('annotate','批注','rebuild','annotation 槽位的新独立稿',p('M3 4h14v8 M3 4v16h8 M7 8h6 M7 12h3 M12 20l1-4 5-5 3 3-5 5-4 1Z M17 12l3 3'),'rebuild','替代已作废 annotation，使用新的文件加笔构形')
add('access-read','只读权限','rebuild','permissionReadOnly 新独立稿',p('M3 12s3-6 9-6 9 6 9 6-3 6-9 6S3 12 3 12Z')+c(12,12,3),'rebuild','替代已作废只读原稿，用眼睛直指查看权限')
add('access-workspace','工作区写入','rebuild','permissionWorkspaceWrite 新独立稿',p('M3 19V6h6l2 3h10v4 M12 21l1-4 5-5 3 3-5 5-4 1Z M17 13l3 3'),'rebuild','替代已作废工作区原稿，文件夹范围加编辑动作')
add('access-full','完整访问','rebuild','permissionFullAccess 新独立稿',p('M12 3 20 6v6c0 4-4.5 7-8 9-3.5-2-8-5-8-9V6l8-3Z M10 12h6 M14 12v3')+c(8.5,12,1.5),'rebuild','替代已作废完整访问原稿，用盾牌内钥匙表达授权')
add('desktop-control','电脑控制','rebuild','Computer Use 新独立稿',p('M11 18H3V4h18v9 M8 22h4 M10 18v4 M14 11v9l3-2 3 2 1-2-4-3 3-1-6-3Z'),'rebuild','替代已作废电脑控制原稿，用显示器加指针表达桌面操作')
assert len(items)==51, len(items)
manifest={'format':'tekesui-icon-proposals/1','status':'review-candidates','style':{'viewBox':'0 0 24 24','stroke':1.5,'caps':'round','color':'currentColor','preview_pt':[12,16,24]},'items':items}
(ROOT/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n')
print(f'{len(items)} SVG proposals: 38 additions, 8 refinements, 5 independent rebuilds')
