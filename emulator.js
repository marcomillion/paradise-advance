'use strict';
// NEW: Standalone browser pet simulator. This is an original gameplay simulation,
// not execution of the proprietary Tamagotchi Paradise firmware.
(function(){
 const canvas=document.getElementById('previewCanvas');
 const empty=document.getElementById('previewEmpty');
 if(!canvas||!empty)return;
 const ctx=canvas.getContext('2d');
 const game={stage:'egg',menu:0,screen:'home',hunger:70,happy:75,clean:90,age:0,hatchedAt:0,eggWobble:0,lastTick:Date.now(),toast:'WELCOME!',toastUntil:Date.now()+2200};
 const menuItems=['FEED','PLAY','CLEAN','STATUS','SLEEP'];
 const saved=(()=>{try{return JSON.parse(localStorage.getItem('paradiseBrowserPet')||'null')}catch{return null}})();
 if(saved&&['egg','pet'].includes(saved.stage)){Object.assign(game,saved);game.lastTick=Date.now()}
 empty.style.display='none';
 function box(x,y,w,h,c){ctx.fillStyle=c;ctx.fillRect(x,y,w,h)}
 function pixelText(t,x,y,color='#17364a',size=8){ctx.fillStyle=color;ctx.font='bold '+size+'px monospace';ctx.textAlign='center';ctx.fillText(t,x,y)}
 function save(){try{localStorage.setItem('paradiseBrowserPet',JSON.stringify({...game,lastTick:Date.now()}))}catch{}}
 function drawEgg(){const wob=Math.sin(Date.now()/180)*2;
  // custom pixel egg silhouette
  const rows=[[58,70,12],[50,78,8],[44,84,7],[40,88,7],[36,92,8],[34,94,8],[32,96,9],[34,94,9],[38,90,9],[44,84,8],[52,76,7],[62,66,5]];
  rows.forEach((r,i)=>box(r[0]+(i>3&&i<8?wob:0),r[1],r[2],4,'#257e99'));
  rows.forEach((r,i)=>box(r[0]+2+(i>3&&i<8?wob:0),r[1]+2,r[2]-4,2,'#e7fcf7'));
  box(50+wob,62,5,4,'#ffffff');box(70+wob,62,5,4,'#ffffff');
  pixelText('?',64,108,'#247a93',10);
 }
 function drawCreature(){const bob=Math.sin(Date.now()/300)*2, y=68+bob;
  // simple water creature made from chunky pixel blocks
  box(46,y,36,26,'#246f9c');box(38,y+8,8,14,'#246f9c');box(82,y+8,8,14,'#246f9c');
  box(50,y-4,28,8,'#246f9c');box(54,y-8,20,6,'#8de5d7');
  box(50,y+4,28,20,'#8de5d7');box(42,y+10,8,8,'#8de5d7');box(78,y+10,8,8,'#8de5d7');
  box(54,y+8,5,6,'#17364a');box(70,y+8,5,6,'#17364a');box(56,y+19,16,2,'#246f9c');
  box(48,y+26,10,4,'#246f9c');box(70,y+26,10,4,'#246f9c');
  box(58,y-12,12,4,'#f5d67e');box(62,y-16,4,4,'#f5d67e');
 }
 function draw(){
  const now=Date.now();ctx.imageSmoothingEnabled=false;box(0,0,128,128,'#dff8f2');
  box(0,0,128,15,'#a5e6dc');pixelText('PARADISE',64,10,'#174e65',8);
  // decorative water bubbles
  [[14,35,3],[106,28,2],[19,65,2],[111,77,3],[99,49,2]].forEach(([x,y,r])=>{box(x-r,y-r,r*2+1,1,'#9bdad9');box(x-r,y+r,r*2+1,1,'#9bdad9');box(x-r,y-r,1,r*2+1,'#9bdad9');box(x+r,y-r,1,r*2+1,'#9bdad9')});
  if(game.stage==='egg')drawEgg();else drawCreature();
  box(4,4,120,10,'#dff8f2');pixelText('DAY '+String(game.age+1).padStart(2,'0'),64,12,'#174e65',7);
  if(game.screen==='menu'){
   box(13,30,102,78,'#f5fffc');box(13,30,102,78,'#5e9eaa');box(16,33,96,72,'#f5fffc');
   pixelText('MENU',64,44,'#247a93',8);
   menuItems.forEach((item,i)=>{if(i===game.menu)box(23,49+i*10,82,9,'#b9eee3');pixelText((i===game.menu?'▶ ':'  ')+item,64,56+i*10,'#174e65',7)});
  }else if(game.screen==='status'){
   box(13,31,102,70,'#5e9eaa');box(16,34,96,64,'#f5fffc');pixelText('STATUS',64,46,'#247a93',8);
   pixelText('HUNGER '+Math.round(game.hunger),64,59,'#174e65',7);pixelText('HAPPY  '+Math.round(game.happy),64,70,'#174e65',7);pixelText('CLEAN  '+Math.round(game.clean),64,81,'#174e65',7);pixelText('AGE    '+(game.age+1)+' DAY',64,92,'#174e65',7);
  }
  box(0,112,128,16,'#a5e6dc');
  if(game.stage==='egg')pixelText('B: HATCH   A: MENU',64,122,'#174e65',7);
  else pixelText(game.screen==='menu'?'A: NEXT  B: OK  C: BACK':'A: MENU   B: ACTION',64,122,'#174e65',6);
  if(game.toastUntil>now){box(17,96,94,13,'#174e65');pixelText(game.toast,64,105,'#ffffff',7)}
 }
 function say(t){game.toast=t;game.toastUntil=Date.now()+1400;draw();save()}
 function hatch(){if(game.stage==='pet')return;game.stage='pet';game.hatchedAt=Date.now();game.screen='home';say('IT HATCHED!')}
 function action(){if(game.stage==='egg'){hatch();return}if(game.screen==='status'){game.screen='home';draw();return}if(game.screen==='menu'){const item=menuItems[game.menu];if(item==='STATUS'){game.screen='status';draw();return}game.screen='home';if(item==='FEED'){game.hunger=Math.min(100,game.hunger+25);say('YUMMY!')}else if(item==='PLAY'){game.happy=Math.min(100,game.happy+20);game.hunger=Math.max(0,game.hunger-8);say('SO FUN!')}else if(item==='CLEAN'){game.clean=100;say('ALL CLEAN!')}else if(item==='SLEEP'){say('NIGHT NIGHT!')}return}game.screen='menu';game.menu=0;draw()}
 function input(d){if(!d)return;if(d.type==='dial'){if(game.screen==='menu'){game.menu=(game.menu+d.steps%menuItems.length+menuItems.length)%menuItems.length;draw()}return}if(d.type!=='button')return;const b=d.button;if(b==='A'){if(game.stage==='egg'){say('PRESS B TO HATCH');return}if(game.screen==='menu'){game.menu=(game.menu+1)%menuItems.length;draw()}else{game.screen='menu';game.menu=0;draw()}}else if(b==='B'||b==='DIAL'){action()}else if(b==='C'){game.screen='home';say('BACK');}}
 document.addEventListener('paradise:device-input',e=>input(e.detail));
 document.addEventListener('keydown',e=>{if(!document.getElementById('view-emulator')?.classList.contains('active'))return;if(['ArrowUp','ArrowDown'].includes(e.key)){input({type:'dial',steps:e.key==='ArrowDown'?1:-1})}});
 setInterval(()=>{const now=Date.now(),elapsed=Math.min(60,(now-game.lastTick)/1000);game.lastTick=now;if(game.stage==='pet'&&game.screen!=='status'){game.hunger=Math.max(0,game.hunger-elapsed*.025);game.happy=Math.max(0,game.happy-elapsed*.018);game.clean=Math.max(0,game.clean-elapsed*.012)}draw();save()},900);
 window.paradiseBrowserPetDraw=draw;
 draw();
})();
