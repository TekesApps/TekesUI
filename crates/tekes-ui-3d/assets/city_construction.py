"""Original dust screens and fictional contractor banners; offline authoring only."""
from PIL import Image,ImageDraw
from block_geometry import Geometry
from city_shopfronts import sign_image


def add_dust_screen(asset):
    # Cutout weave stays in the existing MASK path, avoiding transparent-instance
    # sorting. Coarse seams and hems survive GPU mipmap minification.
    texture=Image.new('RGBA',(128,128),(57,112,102,255));draw=ImageDraw.Draw(texture)
    for y in range(0,128,8):
        for x in range(0,128,8):
            draw.rectangle((x+3,y+3,x+5,y+5),fill=(38,76,70,0))
        draw.line((0,y,127,y),fill=(76,131,116,255),width=1)
    draw.line((2,0,2,127),fill=(91,145,125,255),width=3)
    mesh=asset.material('Original woven construction dust screen',(255,255,255),asset.image(texture),masked=True)
    banner=sign_image('TEKES BUILD',(43,74,98))
    d=ImageDraw.Draw(banner);d.text((112,44),'BUILDING TOMORROW',fill=(235,231,209));d.polygon([(16,48),(34,14),(52,48),(42,48),(34,33),(26,48)],fill=(231,190,101))
    logo=asset.material('Fictional Tekes Build contractor banner',(255,255,255),asset.image(banner))
    g=Geometry(asset)
    for level in range(6):
        y0=.12+level*.8;y1=min(y0+.8,4.22)
        if y0>=y1:continue
        faces=[([(-1.46,y0,1.29),(1.46,y0,1.29),(1.46,y1,1.29),(-1.46,y1,1.29)],2.92),
               ([(1.46,y0,-1.29),(-1.46,y0,-1.29),(-1.46,y1,-1.29),(1.46,y1,-1.29)],2.92),
               ([(1.46,y0,1.29),(1.46,y0,-1.29),(1.46,y1,-1.29),(1.46,y1,1.29)],2.58),
               ([(-1.46,y0,-1.29),(-1.46,y0,1.29),(-1.46,y1,1.29),(-1.46,y1,-1.29)],2.58)]
        for points,width in faces:g.quad(points,mesh,uv=[(0,(y1-y0)*2),(width*2,(y1-y0)*2),(width*2,0),(0,0)])
    for y in [.66,2.7]:
        g.quad([(-1.16,y-.20,1.305),(1.16,y-.20,1.305),(1.16,y+.20,1.305),(-1.16,y+.20,1.305)],logo)
        g.quad([(-1.475,y-.20,-.96),(-1.475,y-.20,.96),(-1.475,y+.20,.96),(-1.475,y+.20,-.96)],logo)
    g.finish('construction-dust-screen')
    asset.nodes[-1]['extras']=dict(original=True,fictional_company='Tekes Build',dust_screen=True,remove_after_completion=True)
