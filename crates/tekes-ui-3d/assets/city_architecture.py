"""Original district architecture: facade atlases, massing and solid relief details.
Offline authoring only; no game textures or runtime animation policy.
"""
import math,random
from PIL import Image,ImageDraw


def facade_atlas(style,seed=0,emission=False):
    # Four bays x four stories: more pixels per window, half the old atlas area.
    im=Image.new('RGB',(512,512));draw=ImageDraw.Draw(im);rng=random.Random(7750+style+seed*101)
    glow=Image.new('RGB',(512,512));gd=ImageDraw.Draw(glow)
    stone=[(157,170,170),(161,108,79),(211,201,174)][style]
    for row in range(4):
        for col in range(4):
            x,y=col*128,row*128
            draw.rectangle((x,y,x+127,y+127),fill=stone)
            if style==1:
                for by in range(0,128,12):
                    for bx in range(-16 if (by//12)%2 else 0,128,32):
                        c=tuple(max(0,min(255,v+rng.randint(-9,9))) for v in stone)
                        draw.rectangle((x+max(0,bx)+1,y+by+1,x+min(127,bx+30),y+by+10),fill=c)
            elif style==2:
                for by in [0,64,127]:draw.line((x,y+by,x+127,y+by),fill=(183,176,155),width=2)
                draw.line((x+64,y,x+64,y+64),fill=(188,179,157),width=1)
            glass=tuple(v+rng.randint(-8,8) for v in [(67,100,119),(72,91,101),(75,101,114)][style])
            draw.rectangle((x+14,y+18,x+114,y+112),fill=(64,67,65))
            draw.rectangle((x+18,y+22,x+110,y+108),fill=glass)
            if style==2:
                draw.rectangle((x+18,y+22,x+110,y+57),fill=stone)
                draw.ellipse((x+18,y+22,x+110,y+88),fill=glass)
                draw.rectangle((x+18,y+56,x+110,y+108),fill=glass)
            rr=random.Random(2983+seed*113+style*17+row*4+col)
            warm=rr.choice([(248,211,151),(232,191,118),(239,220,172),(205,218,219)])
            # Every room has an emission mask; GPU owns occupancy and lighting.
            gd.rectangle((x+18,y+22,x+110,y+108),fill=warm)
            if style==2:
                gd.rectangle((x+18,y+22,x+110,y+57),fill=(0,0,0))
                gd.ellipse((x+18,y+22,x+110,y+88),fill=warm)
                gd.rectangle((x+18,y+56,x+110,y+108),fill=warm)
            # Interior objects and people sit behind glazing, sill and mullions.
            for painter,color in [(draw,(104,105,95)),(gd,(51,40,29))]:

                painter.rectangle((x+24,y+99,x+56,y+104),fill=color)
                painter.rectangle((x+87,y+94,x+105,y+104),fill=color)
            # Recess shading, subdued reflections, frames and occasional blinds.
            draw.line((x+20,y+26,x+20,y+105),fill=(53,68,76),width=3)
            draw.line((x+23,y+105,x+107,y+105),fill=(119,143,150),width=2)
            draw.line((x+63,y+24,x+63,y+107),fill=stone,width=4)
            draw.line((x+20,y+68,x+109,y+68),fill=stone,width=3)
            if rng.random()<.35:
                bottom=y+rng.randint(42,67)
                draw.rectangle((x+22,y+27,x+59,bottom),fill=(159,160,143))
                gd.rectangle((x+22,y+27,x+59,bottom),fill=(35,30,23))
            gd.line((x+63,y+22,x+63,y+108),fill=(0,0,0),width=4)
            gd.line((x+18,y+68,x+110,y+68),fill=(0,0,0),width=3)
            draw.line((x+13,y+114,x+115,y+114),fill=(109,114,109),width=3)
    return glow if emission else im


def segments(w,d,floors,style,name):
    setback=0 if style==1 else 2 if style==0 else 1
    inset=0 if not setback else .50 if style==0 else .40 if 'hotel' in name.lower() else .36
    if not setback:return [(w,d,floors)]
    return [(w,d,floors-setback),(w-inset,d-inset,setback)]


def relief(g,plain,x,z,w,d,base,floors,floor_height,style,balconies=False,blocked_faces=()):
    frame=[(111,130,134),(204,185,152),(226,216,187)][style]
    belt=[(138,152,153),(178,154,123),(203,192,165)][style]
    def box(nx,nz,u,y,v,bw,bh,bd,color):
        tx,tz=nz,-nx;extent=d/2 if nz else w/2
        g.box(x+tx*u+nx*(extent+v),y,z+tz*u+nz*(extent+v),bw if nz else bd,bh,bd if nz else bw,plain,color)
    for level in range(floors):
        y=base+level*floor_height
        for nx,nz in [(0,1),(1,0),(0,-1),(-1,0)]:
            if (nx,nz) in blocked_faces:continue
            width=w if nz else d;bays=max(1,round(width/.5));pitch=width/bays;ww=min(.36,pitch*.78)
            box(nx,nz,0,y+.015,.013,width+.035,.035,.035,belt)
            for bay in range(bays):
                u=(bay+.5)*pitch-width/2
                box(nx,nz,u,y+.115,.035,ww+.06,.035,.085,frame)
                for edge in [-1,1]:box(nx,nz,u+edge*ww/2,y+.39,.018,.016,.54,.035,frame)
                box(nx,nz,u,y+.39,.015,.012,.54,.027,frame)
                box(nx,nz,u,y+.40,.015,ww,.012,.027,frame)
                if style==2:
                    for j in range(8):
                        t0=j*math.pi/8;t1=(j+1)*math.pi/8;extent=d/2 if nz else w/2;tx,tz=nz,-nx
                        points=[]
                        for t in [t0,t1]:
                            uu=u+math.cos(t)*ww/2;yy=y+.45+math.sin(t)*.20
                            points.append((x+tx*uu+nx*(extent+.020),yy,z+tz*uu+nz*(extent+.020)))
                        g.beam(points[0],points[1],.018,plain,frame)
                else:box(nx,nz,u,y+.66,.022,ww+.035,.025,.045,frame)
            # Corner quoins / pilasters provide an actual silhouette and changing normals.
            for edge in [-1,1]:box(nx,nz,edge*(width/2-.045),y+.40,.018,.09,.76,.055,frame)
        if balconies:
            for u in [-w*.26,w*.26]:
                box(0,1,u,y+.035,.13,.60,.045,.29,(169,156,132))
                box(0,1,u,y+.27,.255,.60,.025,.024,(84,102,101))
                for j in range(5):box(0,1,u-.27+j*.135,y+.15,.255,.014,.23,.014,(84,102,101))
    top=base+floors*floor_height
    for nx,nz in [(0,1),(1,0),(0,-1),(-1,0)]:
        width=w if nz else d
        box(nx,nz,0,top-.025,.035,width+.08,.045,.085,frame)
        box(nx,nz,0,top+.012,.045,width+.10,.025,.10,frame)


def ground_relief(g,plain,x,z,w,d,base,entrance_side,blocked_faces=()):
    """Courtyard/service windows add detail without inventing extra entrances."""
    entry={'north':(0,-1),'east':(1,0),'south':(0,1),'west':(-1,0)}[entrance_side]
    for nx,nz in [(0,1),(1,0),(0,-1),(-1,0)]:
        if (nx,nz)==entry or (nx,nz) in blocked_faces:continue
        width=w if nz else d;extent=d/2 if nz else w/2;tx,tz=nz,-nx
        def box(u,y,v,bw,bh,bd,color):
            g.box(x+tx*u+nx*(extent+v),base+y,z+tz*u+nz*(extent+v),bw if nz else bd,bh,bd if nz else bw,plain,color)
        box(0,.09,.016,width,.10,.036,(162,159,143))
        bays=max(2,round(width/.65));pitch=width/bays
        for bay in range(bays):
            u=(bay+.5)*pitch-width/2
            box(u,.50,-.18,.28,.42,.016,(57,86,99))
            for sign in [-1,1]:box(u+sign*.15,.50,.025,.022,.46,.04,(186,187,168))
            box(u,.29,.035,.34,.035,.08,(193,190,170))
            box(u,.73,.025,.34,.035,.045,(193,190,170))
            box(u,.50,.023,.012,.42,.027,(160,172,168))
