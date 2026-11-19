"""Original street-level shells and shopfronts, baked into independent building meshes."""
from pathlib import Path
import random
from PIL import Image,ImageDraw,ImageFont

DIRECTIONS={'north':(0,-1),'east':(1,0),'south':(0,1),'west':(-1,0)}


def sign_image(text,color):
    im=Image.new('RGB',(384,64),color);draw=ImageDraw.Draw(im)
    font_path=Path('/System/Library/Fonts/Supplemental/Arial Bold.ttf')
    font=ImageFont.truetype(str(font_path),26) if font_path.exists() else ImageFont.load_default()
    bounds=draw.textbbox((0,0),text,font=font);width=bounds[2]-bounds[0]
    draw.text(((384-width)/2,15),text,font=font,fill=(235,231,209))
    draw.line((10,6,373,6),fill=(178,174,150),width=1)
    draw.line((10,57,373,57),fill=(178,174,150),width=1)
    return im


def masonry_image():
    im=Image.new('RGB',(256,256),(215,213,202));draw=ImageDraw.Draw(im);rng=random.Random(87193)
    for row in range(8):
        for col in range(-1,5):
            x=col*64+(32 if row%2 else 0);y=row*32
            value=rng.randint(234,247)
            draw.rectangle((max(0,x+1),y+1,min(255,x+63),y+30),fill=(value,value-1,value-5)) if x+63>=0 and x+1<256 else None
            if 0<=x+2<256:draw.line((x+2,y+2,min(255,x+62),y+2),fill=(250,249,240),width=1)
    return im


def ground_storey(g,plain,x,z,w,d,base,side,color,garage,floor_height=.95,wall_material=None):
    # Structural core and four aperture walls: openings have actual recessed depth.
    g.box(x,base+floor_height/2,z,w-.40,floor_height,d-.40,plain,color,facade=wall_material,facade_floor_height=.125,facade_bay_width=.25,facade_grid=(4,8))
    for face,(nx,nz) in DIRECTIONS.items():
        tx,tz=nz,-nx;width=w if nz else d;extent=d/2 if nz else w/2
        if face==side:
            openings=[(-.17,.17,0,.75)]
            for u in [-width*.28,width*.28]:
                if not garage or u<0:openings.append((u-width*.16,u+width*.16,.075,.725))
            if garage:
                u=width*.28;openings.append((u-.38,u+.38,0,.60))
        else:
            bays=max(2,round(width/.65));pitch=width/bays
            openings=[((j+.5)*pitch-width/2-.14,(j+.5)*pitch-width/2+.14,.29,.71) for j in range(bays)]
        def panel(lo,hi,bottom,top):
            if hi-lo<1e-5 or top-bottom<1e-5:return
            u=(lo+hi)/2;v=extent-.10
            g.box(x+tx*u+nx*v,base+(bottom+top)/2,z+tz*u+nz*v,hi-lo if nz else .20,top-bottom,.20 if nz else hi-lo,plain,color,facade=wall_material,facade_floor_height=.125,facade_bay_width=.25,facade_grid=(4,8))
        edge=-width/2
        for lo,hi,bottom,top in sorted(openings):
            assert lo>=edge-1e-5 and hi<=width/2
            panel(edge,lo,0,floor_height);panel(lo,hi,0,bottom);panel(lo,hi,top,floor_height);edge=hi
        panel(edge,width/2,0,floor_height)


def entrance(g,plain,x,z,w,d,base,side,door_color,awning_color,garage=False,sign_material=None,kind='lobby'):
    nx,nz=DIRECTIONS[side];tx,tz=nz,-nx;extent=d/2 if nz else w/2;width=w if nz else d
    px,pz=x+nx*extent,z+nz*extent
    def point(u,y,v):return (px+tx*u+nx*v,base+y,pz+tz*u+nz*v)
    def box(u,y,v,bw,bh,bd,color):g.box(*point(u,y,v),bw if nz else bd,bh,bd if nz else bw,plain,color)
    def cylinder(u,y,v,r,h,color):g.cylinder(*point(u,y,v),r,h,plain,color)
    # Paving supports furniture and joins the doorway; the garage path remains clear.
    ranges=[(-width/2,width/2)] if not garage else [(-width/2,width*.28-.42),(width*.28+.42,width/2)]
    for lo,hi in ranges:
        if hi>lo:box((lo+hi)/2,-.08,.27,hi-lo,.16,.54,(181,177,158))
    for u in [-width*.28,width*.28]:
        if garage and u>0:continue
        span=width*.32
        box(u,.40,-.182,span,.65,.024,(45,66,72))
        box(u,.085,-.10,span,.025,.20,(133,126,109))
        for edge in [-1,1]:box(u+edge*span/2,.40,.014,.025,.69,.05,(165,166,148))
        box(u,.735,.014,span+.05,.03,.05,(177,179,159))
        box(u,.075,.025,span+.05,.03,.09,(184,180,159))
        box(u,.40,.010,.018,.65,.04,(128,145,144))
        # Actual shallow interior: shelves/counter and varied objects behind the opening.
        for shelf in [.20,.39]:
            box(u,shelf,-.10,span*.86,.018,.14,(166,145,108))
            for j in range(4):
                du=(j-1.5)*span*.18;color=[(174,120,67),(113,147,106),(184,168,105),(126,145,159)][j]
                box(u+du,shelf+.055,-.105,span*.12,.09,.075,color)
        if kind in ['office','hotel']:
            box(u,.23,-.075,span*.70,.26,.12,(144,121,92))
            cylinder(u+span*.27,.38,-.09,.035,.07,(121,112,87))
            g.ellipsoid(*point(u+span*.27,.45,-.09),.065,.09,.055,plain,(75,121,76),sides=10,rings=6)
        # Restrained glass reflections leave most of the display visible.
        box(u-span*.31,.44,-.015,.012,.45,.008,(146,170,176))
    box(0,.28,-.12,.27,.56,.04,door_color)
    box(0,.34,-.094,.22,.34,.012,(53,80,91))
    box(0,.04,-.091,.26,.065,.018,(157,161,149))
    for edge in [-1,1]:box(edge*.145,.375,.018,.022,.75,.05,(154,162,153))
    box(0,.565,.018,.31,.025,.05,(177,179,165))
    box(0,.66,-.10,.27,.16,.025,(67,94,102))
    box(0,.75,.018,.34,.025,.05,(177,179,165))
    box(.075,.29,-.066,.012,.075,.018,(186,179,146))
    box(0,.005,-.085,.32,.018,.22,(168,164,147))
    box(0,.82,.18,width*.92,.06,.4,awning_color)
    for u in [-width*.40,0,width*.40]:
        g.beam(point(u,.88,.015),point(u,.795,.35),.018,plain,(92,108,105))
        cylinder(u,.777,.27,.018,.012,(234,224,185))
    if sign_material is not None:
        sw=min(width*.70,1.25)
        pts=[point(-sw/2,.75,.385),point(sw/2,.75,.385),point(sw/2,.91,.385),point(-sw/2,.91,.385)]
        g.quad(pts,sign_material,(255,255,255),normals=[(nx,0,nz)]*4)
    if kind=='cafe':
        for u in [-width*.28,width*.28]:
            cylinder(u,.18,.06,.055,.018,(167,142,107));cylinder(u,.085,.06,.015,.17,(76,95,94))
            for cup in [-.025,.025]:cylinder(u+cup,.205,.06,.012,.03,(222,215,185))
            for offset in [-.13,.13]:
                box(u+offset,.11,.06,.12,.022,.10,(158,133,99))
                box(u+offset,.21,.10,.12,.18,.018,(158,133,99))
                for leg in [-.045,.045]:
                    for depth in [-.045,.045]:box(u+offset+leg,.05,.06+depth,.014,.10,.014,(73,91,89))
    entry=dict(side=side,normal=[nx,0,nz],position=[px,base,pz],door_color=door_color,door_plane_offset=-.12,recess_depth=.20)
    if garage:
        u=width*.28
        box(u,.30,-.185,.76,.60,.018,(30,39,43))
        for y in [.52,.56,.60]:box(u,y,-.17,.74,.022,.025,(118,130,130))
        box(u,.70,.024,.17,.16,.025,(45,90,137))
        for du,dy,bw,bh in [(-.033,.70,.018,.12),(.003,.75,.075,.016),(.003,.705,.075,.016),(.035,.727,.016,.06)]:box(u+du,dy,.042,bw,bh,.012,(231,234,218))
        box(u,.012,-.10,.76,.024,.28,(138,145,139))
        entry['garage']=dict(position=[px+tx*u,base,pz+tz*u],width=.76,height=.60)
    return entry
