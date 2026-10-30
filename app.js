'use strict';
const $=id=>document.getElementById(id);
const state={sprites:[],activeId:null,tool:'pencil',color:'#211c36',grid:true,zoom:10,history:[],redo:[],firmware:null,gallery:[]};
const animationState={frames:[],selected:-1,timer:null};
const starterPalette=['#211c36','#f4f2ff','#ff9fc8','#9bf2d2','#a99bff','#ffdb91','#5677a8','#e6f6ee'];

const WORKSPACE_STORAGE_KEY='paradiseModStudio.workspace.v1';
let workspaceSaveTimer=null,workspaceStorageWarningShown=false;
function validSavedSprite(s){
 return s&&typeof s==='object'&&Number.isInteger(s.width)&&Number.isInteger(s.height)&&s.width>=1&&s.height>=1&&s.width<=256&&s.height<=256&&Array.isArray(s.pixels)&&s.pixels.length===s.width*s.height;
}
function restoreWorkspace(){
 try{
  const raw=localStorage.getItem(WORKSPACE_STORAGE_KEY);if(!raw)return;
  const saved=JSON.parse(raw);if(!saved||saved.version!==1)return;
  const normalise=list=>list.filter(validSavedSprite).slice(0,200).map(s=>({...s,id:String(s.id||uid()),name:String(s.name||'Untitled Sprite').slice(0,64),pixels:s.pixels.map(c=>typeof c==='string'&&/^#[0-9a-f]{6}$/i.test(c)?c:null),palette:Array.isArray(s.palette)?s.palette.filter(c=>typeof c==='string'&&/^#[0-9a-f]{6}$/i.test(c)).slice(0,64):[...starterPalette]}));
  if(Array.isArray(saved.sprites))state.sprites=normalise(saved.sprites);
  if(Array.isArray(saved.gallery))state.gallery=normalise(saved.gallery);
  state.activeId=state.sprites.some(s=>s.id===saved.activeId)?saved.activeId:(state.sprites[0]?.id||null);
  if(typeof saved.color==='string'&&/^#[0-9a-f]{6}$/i.test(saved.color))state.color=saved.color;
  if(typeof saved.grid==='boolean')state.grid=saved.grid;
  if(Number.isFinite(saved.zoom))state.zoom=Math.max(4,Math.min(20,saved.zoom));
 }catch(_){/* Storage can be disabled or contain an old/corrupt workspace. */}
}
function scheduleWorkspaceSave(){
 clearTimeout(workspaceSaveTimer);
 workspaceSaveTimer=setTimeout(()=>{
  try{
   const payload={version:1,sprites:state.sprites,gallery:state.gallery,activeId:state.activeId,color:state.color,grid:state.grid,zoom:state.zoom};
   localStorage.setItem(WORKSPACE_STORAGE_KEY,JSON.stringify(payload));
  }catch(_){
   if(!workspaceStorageWarningShown){workspaceStorageWarningShown=true;toast('Browser storage is full or unavailable. Export your sprite JSON to keep a backup.');}
  }
 },250);
}

function uid(){return 's'+Date.now().toString(36)+Math.random().toString(36).slice(2,7)}
function toast(s){const t=$('toast');t.textContent=s;t.classList.add('show');clearTimeout(toast.timer);toast.timer=setTimeout(()=>t.classList.remove('show'),2800)}
function active(){return state.sprites.find(s=>s.id===state.activeId)||null}
function createSprite(name='Untitled Sprite',w=32,h=32,pixels=null,palette=null){w=Math.max(1,Math.min(256,Number(w)||32));h=Math.max(1,Math.min(256,Number(h)||32));const s={id:uid(),name,width:w,height:h,pixels:pixels&&pixels.length===w*h?pixels:Array(w*h).fill(null),palette:palette||[...starterPalette]};state.sprites.push(s);state.activeId=s.id;state.history=[];state.redo=[];renderAll();return s}
function snap(){const s=active();if(!s)return;state.history.push(JSON.stringify(s));if(state.history.length>80)state.history.shift();state.redo=[]}
function restore(raw){const old=JSON.parse(raw);const i=state.sprites.findIndex(s=>s.id===old.id);if(i>=0)state.sprites[i]=old;state.activeId=old.id;renderAll()}
function undo(){if(!state.history.length)return toast('Nothing to undo');const s=active();if(!s)return;state.redo.push(JSON.stringify(s));restore(state.history.pop())}
function redo(){if(!state.redo.length)return toast('Nothing to redo');const s=active();if(!s)return;state.history.push(JSON.stringify(s));restore(state.redo.pop())}
function drawSprite(canvas,s,scale=1,bg=null,grid=false){const ctx=canvas.getContext('2d');canvas.width=s.width;canvas.height=s.height;ctx.clearRect(0,0,canvas.width,canvas.height);if(bg){ctx.fillStyle=bg;ctx.fillRect(0,0,canvas.width,canvas.height)}for(let y=0;y<s.height;y++)for(let x=0;x<s.width;x++){const c=s.pixels[y*s.width+x];if(c){ctx.fillStyle=c;ctx.fillRect(x,y,1,1)}}if(grid){ctx.strokeStyle='#000000';ctx.lineWidth=.14;ctx.beginPath();for(let x=0;x<=s.width;x++){ctx.moveTo(x,0);ctx.lineTo(x,s.height)}for(let y=0;y<=s.height;y++){ctx.moveTo(0,y);ctx.lineTo(s.width,y)}ctx.stroke()}}
function renderCanvas(){const s=active(),c=$('spriteCanvas'),z=state.zoom;if(!s){const w=32,h=32;c.width=w*z;c.height=h*z;const ctx=c.getContext('2d');ctx.fillStyle='#ffffff';ctx.fillRect(0,0,c.width,c.height);if(state.grid){ctx.beginPath();ctx.strokeStyle='#000000';ctx.lineWidth=1;for(let x=0;x<=w;x++){const px=x*z+.5;ctx.moveTo(px,0);ctx.lineTo(px,c.height)}for(let y=0;y<=h;y++){const py=y*z+.5;ctx.moveTo(0,py);ctx.lineTo(c.width,py)}ctx.stroke()}c.style.width=(w*z)+'px';c.style.height=(h*z)+'px';$('canvasHint').style.display='block';return}$('canvasHint').style.display='none';c.width=s.width*z;c.height=s.height*z;const ctx=c.getContext('2d');ctx.fillStyle='#ffffff';ctx.fillRect(0,0,c.width,c.height);for(let y=0;y<s.height;y++)for(let x=0;x<s.width;x++){const col=s.pixels[y*s.width+x];if(col){ctx.fillStyle=col;ctx.fillRect(x*z,y*z,z,z)}}if(state.grid){ctx.beginPath();ctx.strokeStyle='#000000';ctx.lineWidth=1;for(let x=0;x<=s.width;x++){const px=x*z+.5;ctx.moveTo(px,0);ctx.lineTo(px,c.height)}for(let y=0;y<=s.height;y++){const py=y*z+.5;ctx.moveTo(0,py);ctx.lineTo(c.width,py)}ctx.stroke()}c.style.width=(s.width*z)+'px';c.style.height=(s.height*z)+'px';$('currentSpriteName').textContent=s.name.toUpperCase();$('spriteName').value=s.name;$('spriteWidth').value=s.width;$('spriteHeight').value=s.height;$('dimensions').textContent=`${s.width} × ${s.height}`;renderPalette();renderPreview()}
function renderPalette(){const s=active(),p=$('palette');p.innerHTML='';(s?s.palette:starterPalette).forEach(col=>{const b=document.createElement('button');b.className='swatch'+(col.toLowerCase()===state.color.toLowerCase()?' active':'');b.style.background=col;b.title=col;b.onclick=()=>setColor(col);p.append(b)})}
function setColor(c){state.color=c;$('colorPicker').value=c;renderPalette();scheduleWorkspaceSave()}
function renderAssets(){const list=$('assetList');list.innerHTML='';$('spriteCount').textContent=state.sprites.length;if(!state.sprites.length){list.innerHTML='<div class="empty-state">No sprites loaded yet.<br><small>Import a PNG or create a blank sprite.</small></div>';return}for(const s of state.sprites){const b=document.createElement('button');b.className='asset-item'+(s.id===state.activeId?' active':'');const cv=document.createElement('canvas');cv.className='asset-thumb';cv.width=s.width;cv.height=s.height;drawSprite(cv,s);const d=document.createElement('div'),strong=document.createElement('strong'),small=document.createElement('small');strong.textContent=s.name;small.textContent=`${s.width} × ${s.height}`;d.append(strong,small);b.append(cv,d);b.onclick=()=>{state.activeId=s.id;renderAll()};list.append(b)}}
function renderPreview(){if(typeof window.paradiseBrowserPetDraw==='function'){window.paradiseBrowserPetDraw();return}const c=$('previewCanvas');const ctx=c.getContext('2d');ctx.clearRect(0,0,c.width,c.height);ctx.fillStyle='#f6fdff';ctx.fillRect(0,0,c.width,c.height);$('previewEmpty').style.display='grid';$('previewEmpty').innerHTML='128 × 128 DISPLAY<br>CORE NOT CONNECTED';if(!state.firmware){applyParadiseEdition({edition:'water',confidence:'default'});}}
function renderAll(){renderAssets();renderCanvas();renderGallery();scheduleWorkspaceSave()}
function canvasPos(e){const c=$('spriteCanvas'),r=c.getBoundingClientRect(),s=active();return {x:Math.floor((e.clientX-r.left)/r.width*s.width),y:Math.floor((e.clientY-r.top)/r.height*s.height)}}
function applyPixel(x,y){const s=active();if(!s||x<0||y<0||x>=s.width||y>=s.height)return;const i=y*s.width+x;if(state.tool==='picker'){const c=s.pixels[i];if(c)setColor(c);return}if(state.tool==='fill'){const old=s.pixels[i],next=state.color;if(old===next)return;snap();const stack=[[x,y]],seen=new Set();while(stack.length){const [a,b]=stack.pop(),k=b*s.width+a;if(a<0||b<0||a>=s.width||b>=s.height||seen.has(k)||s.pixels[k]!==old)continue;seen.add(k);s.pixels[k]=next;stack.push([a+1,b],[a-1,b],[a,b+1],[a,b-1])}renderAll();return}if(s.pixels[i]===(state.tool==='eraser'?null:state.color))return;snap();s.pixels[i]=state.tool==='eraser'?null:state.color;renderCanvas();renderAssets()}
let painting=false,paintKey='';$('spriteCanvas').addEventListener('pointerdown',e=>{if(!active())return;const p=canvasPos(e);if(state.tool==='picker'||state.tool==='fill'){applyPixel(p.x,p.y);return}painting=true;paintKey='';$('spriteCanvas').setPointerCapture(e.pointerId);snap();applyPixelNoSnap(p.x,p.y);});$('spriteCanvas').addEventListener('pointermove',e=>{if(!painting)return;const p=canvasPos(e);applyPixelNoSnap(p.x,p.y)});window.addEventListener('pointerup',()=>{if(painting){painting=false;renderAll()}});
function applyPixelNoSnap(x,y){const s=active();if(!s||x<0||y<0||x>=s.width||y>=s.height)return;const i=y*s.width+x;s.pixels[i]=state.tool==='eraser'?null:state.color;renderCanvas()}
function setTool(t){state.tool=t;document.querySelectorAll('.tool').forEach(b=>b.classList.toggle('active',b.dataset.tool===t))}
function nav(view){document.querySelectorAll('.tab').forEach(b=>b.classList.toggle('active',b.dataset.view===view));document.querySelectorAll('.view').forEach(v=>v.classList.toggle('active',v.id==='view-'+view));if(view==='emulator')renderPreview()}
function exportBlob(blob,name){const a=document.createElement('a');a.href=URL.createObjectURL(blob);a.download=name;document.body.append(a);a.click();a.remove();setTimeout(()=>URL.revokeObjectURL(a.href),1000)}
function exportPng(s=active(),filename){if(!s)return toast('Create or select a sprite first');const c=document.createElement('canvas');drawSprite(c,s);c.toBlob(blob=>exportBlob(blob,filename||s.name.replace(/[^a-z0-9_-]+/gi,'_')+'.png'))}
function exportJson(s=active()){if(!s)return toast('Create or select a sprite first');exportBlob(new Blob([JSON.stringify({format:'paradise-mod-studio-sprite',version:1,name:s.name,width:s.width,height:s.height,palette:s.palette,pixels:s.pixels},null,2)],{type:'application/json'}),s.name.replace(/[^a-z0-9_-]+/gi,'_')+'.json')}
async function loadSpriteFile(file,addToGallery=false){if(!file)return;try{if(file.name.toLowerCase().endsWith('.json')){const data=JSON.parse(await file.text());if(data.format==='paradise-mod-studio-animation'){if(addToGallery)throw Error('Import animation JSON from Sprite Studio, not the gallery.');if(!Array.isArray(data.frames)||!data.frames.length||data.frames.length>300)throw Error('Animation JSON must contain 1–300 frames.');let totalPixels=0;const frames=data.frames.map((f,i)=>{if(!Number.isInteger(f.width)||!Number.isInteger(f.height)||f.width<1||f.height<1||f.width>256||f.height>256||!Array.isArray(f.pixels)||f.pixels.length!==f.width*f.height)throw Error('Frame '+(i+1)+' has invalid dimensions or pixels.');totalPixels+=f.pixels.length;if(totalPixels>1000000)throw Error('This animation is too large to import safely.');return{name:String(f.name||('Frame '+(i+1))).slice(0,64),width:f.width,height:f.height,pixels:f.pixels.map(c=>typeof c==='string'&&/^#[0-9a-f]{6}$/i.test(c)?c:null),palette:Array.isArray(f.palette)?f.palette.filter(c=>typeof c==='string'&&/^#[0-9a-f]{6}$/i.test(c)).slice(0,64):[...starterPalette]}});animationState.frames=frames;animationState.selected=0;stopAnimation();$('animationFps').value=String(Math.max(1,Math.min(12,Number(data.fps)||4)));$('animationFpsValue').textContent=$('animationFps').value;renderAnimationStrip();loadAnimationFrame(0);toast('Animation project imported');return}if(data.format!=='paradise-mod-studio-sprite'||!Number.isInteger(data.width)||!Number.isInteger(data.height)||data.width<1||data.height<1||data.width>256||data.height>256||!Array.isArray(data.pixels)||data.pixels.length!==data.width*data.height)throw Error('This JSON is not a valid Studio sprite project. Dimensions must be between 1 × 1 and 256 × 256.');const s={id:uid(),name:String(data.name||file.name).slice(0,64),width:data.width,height:data.height,pixels:data.pixels.map(c=>typeof c==='string'&&/^#[0-9a-f]{6}$/i.test(c)?c:null),palette:Array.isArray(data.palette)?data.palette.filter(c=>typeof c==='string'&&/^#[0-9a-f]{6}$/i.test(c)).slice(0,64):[...starterPalette]};if(addToGallery)state.gallery.unshift(s);else{state.sprites.push(s);state.activeId=s.id}renderAll();toast('Sprite project imported locally')}else{const img=new Image();img.src=URL.createObjectURL(file);await img.decode();if(img.width>256||img.height>256)throw Error('Sprite images must be 256 × 256 pixels or smaller.');const c=document.createElement('canvas');c.width=img.width;c.height=img.height;const ctx=c.getContext('2d',{willReadFrequently:true});ctx.drawImage(img,0,0);const raw=ctx.getImageData(0,0,c.width,c.height).data,pal=[],pixels=[];for(let i=0;i<raw.length;i+=4){if(raw[i+3]<128){pixels.push(null);continue}const col='#'+[raw[i],raw[i+1],raw[i+2]].map(v=>v.toString(16).padStart(2,'0')).join('');pixels.push(col);if(!pal.includes(col)&&pal.length<64)pal.push(col)}const s={id:uid(),name:file.name.replace(/\.[^.]+$/,'').slice(0,64),width:c.width,height:c.height,pixels,palette:pal.length?pal:[...starterPalette]};URL.revokeObjectURL(img.src);if(addToGallery)state.gallery.unshift(s);else{state.sprites.push(s);state.activeId=s.id}renderAll();toast('PNG imported locally')}}catch(e){toast(e.message||'Could not import that file')}}
function renderGallery(){const grid=$('galleryGrid');grid.innerHTML='';if(!state.gallery.length){grid.innerHTML='<div class="empty-state wide">The gallery is empty.<br><small>Add a PNG or project JSON to create your first local card.</small></div>';return}state.gallery.forEach(s=>{const card=document.createElement('article');card.className='gallery-card';const art=document.createElement('div');art.className='gallery-art';const cv=document.createElement('canvas');drawSprite(cv,s);cv.style.width=Math.min(128,s.width*3)+'px';cv.style.height=Math.min(128,s.height*3)+'px';art.append(cv);const h=document.createElement('h3');h.textContent=s.name;const p=document.createElement('p');p.textContent=`${s.width} × ${s.height} · local draft`;const add=document.createElement('button');add.className='button';add.textContent='Open in Studio';add.onclick=()=>{const copy={...s,id:uid(),pixels:[...s.pixels],palette:[...s.palette]};state.sprites.push(copy);state.activeId=copy.id;nav('studio');renderAll();toast('Copied to Sprite Studio')};const exp=document.createElement('button');exp.className='button';exp.textContent='Export PNG';exp.onclick=()=>exportPng(s);const json=document.createElement('button');json.className='button';json.textContent='Export project JSON';json.onclick=()=>exportJson(s);const remove=document.createElement('button');remove.className='button';remove.textContent='Remove draft';remove.onclick=()=>{state.gallery=state.gallery.filter(item=>item.id!==s.id);renderAll();toast('Local draft removed')};card.append(art,h,p,add,exp,json,remove);grid.append(card)})}
function detectParadiseEdition(fileName,bytes){const name=String(fileName||'').toLowerCase();const ascii=[];let run='';for(const byte of bytes){if(byte>=32&&byte<=126){run+=String.fromCharCode(byte);if(run.length>256)run=run.slice(-256)}else{if(run.length>=4)ascii.push(run.toLowerCase());run=''}}if(run.length>=4)ascii.push(run.toLowerCase());const text=name+' '+ascii.join(' ');const tests=[['jade',/jade[ _-]?forest|jadeforest/],['water',/water[ _-]?(edition|ver|version)?|tamagotchi[ _-]?paradise[ _-]?water/],['land',/land[ _-]?(edition|ver|version)?|tamagotchi[ _-]?paradise[ _-]?land/],['sky',/sky[ _-]?(edition|ver|version)?|tamagotchi[ _-]?paradise[ _-]?sky/]];for(const [edition,regex] of tests)if(regex.test(text))return {edition,confidence:'matched'};return {edition:'water',confidence:'default'}}
function applyParadiseEdition(result){const shell=document.querySelector('.device-shell');if(!shell)return;shell.dataset.edition=result.edition;const names={water:'Water',land:'Land',sky:'Sky',jade:'Jade Forest'};const label=names[result.edition]||'Water';const caption=document.querySelector('.device-caption');if(caption)caption.textContent='TAMAGOTCHI PARADISE · '+label.toUpperCase()+(result.confidence==='default'?' · DEFAULT SHELL':'');}
async function loadFirmware(file){if(!file)return;const buf=await file.arrayBuffer(),bytes=new Uint8Array(buf),header=Array.from(bytes.slice(0,64)).map(v=>v.toString(16).padStart(2,'0')).join(' ').match(/.{1,47}/g)?.join('\n')||'Empty file';let hash='Unavailable';if(crypto.subtle){const digest=await crypto.subtle.digest('SHA-256',buf);hash=Array.from(new Uint8Array(digest)).map(v=>v.toString(16).padStart(2,'0')).join('')}const edition=detectParadiseEdition(file.name,bytes);state.firmware={name:file.name,size:bytes.length,bytes,hash,edition};$('firmwareName').textContent=file.name;$('firmwareSummary').textContent='Loaded in memory for this browser session. This page does not upload or patch the source file.';$('firmwareSize').textContent=(bytes.length/1048576).toFixed(2)+' MiB';$('firmwareHeader').textContent=Array.from(bytes.slice(0,8)).map(v=>v.toString(16).padStart(2,'0')).join(' ').toUpperCase();$('firmwareHash').textContent=hash;$('headerDump').textContent=header;const block=65536,regions=[];for(let off=0;off<bytes.length;off+=block){const chunk=bytes.subarray(off,Math.min(off+block,bytes.length));let ff=0,zero=0;for(const v of chunk){if(v===255)ff++;if(v===0)zero++}regions.push({ff:ff/chunk.length,zero:zero/chunk.length})}const map=$('regionMap');map.innerHTML='';regions.forEach((r,i)=>{const el=document.createElement('div');el.className='region-block';el.title=`0x${(i*block).toString(16).toUpperCase()} · FF ${(r.ff*100).toFixed(0)}% · 00 ${(r.zero*100).toFixed(0)}%`;el.style.opacity=String(.25+Math.max(r.ff,r.zero)*.75);map.append(el)});applyParadiseEdition(edition);toast('Firmware loaded for local inspection')}
$('loadFirmwareBtn').onclick=$('firmwareLoadAgain').onclick=()=>$('firmwareInput').click();$('firmwareInput').onchange=e=>loadFirmware(e.target.files[0]);$('importSpriteBtn').onclick=()=>$('spriteInput').click();$('spriteInput').onchange=e=>loadSpriteFile(e.target.files[0]);$('galleryImportBtn').onclick=()=>$('galleryInput').click();$('galleryInput').onchange=e=>loadSpriteFile(e.target.files[0],true);$('newSpriteBtn').onclick=()=>createSprite();$('exportPngBtn').onclick=()=>exportPng();$('exportJsonBtn').onclick=()=>exportJson();$('undoBtn').onclick=undo;$('redoBtn').onclick=redo;$('clearBtn').onclick=()=>{const s=active();if(!s)return toast('Create or select a sprite first');snap();s.pixels.fill(null);renderAll()};$('flipBtn').onclick=()=>{const s=active();if(!s)return; snap();for(let y=0;y<s.height;y++)for(let x=0;x<Math.floor(s.width/2);x++){const a=y*s.width+x,b=y*s.width+(s.width-1-x);[s.pixels[a],s.pixels[b]]=[s.pixels[b],s.pixels[a]]}renderAll()};$('resizeBtn').onclick=()=>{const s=active();if(!s)return toast('Create or select a sprite first');const w=Math.max(1,Math.min(256,Number($('spriteWidth').value)||s.width)),h=Math.max(1,Math.min(256,Number($('spriteHeight').value)||s.height));snap();const next=Array(w*h).fill(null);for(let y=0;y<Math.min(h,s.height);y++)for(let x=0;x<Math.min(w,s.width);x++)next[y*w+x]=s.pixels[y*s.width+x];s.width=w;s.height=h;s.pixels=next;renderAll()};$('spriteName').onchange=()=>{const s=active();if(s){s.name=$('spriteName').value.trim()||'Untitled Sprite';renderAssets();$('currentSpriteName').textContent=s.name.toUpperCase();scheduleWorkspaceSave()}};$('zoomSlider').oninput=e=>{state.zoom=Number(e.target.value);$('zoomValue').textContent=state.zoom+'×';renderCanvas();scheduleWorkspaceSave()};$('gridToggle').onchange=e=>{state.grid=e.target.checked;renderCanvas();scheduleWorkspaceSave()};$('colorPicker').oninput=e=>setColor(e.target.value);$('addColorBtn').onclick=()=>{const s=active();if(s&&!s.palette.includes(state.color)){s.palette.push(state.color);renderPalette()}};document.querySelectorAll('.tool').forEach(b=>b.onclick=()=>setTool(b.dataset.tool));document.querySelectorAll('.tab').forEach(b=>b.onclick=()=>nav(b.dataset.view));
// Device controls emit a stable event contract for the emulator core.
function reportDeviceInput(message, detail){
 const out=$('deviceInputStatus');if(out)out.textContent=message;
 document.dispatchEvent(new CustomEvent('paradise:device-input',{detail}));
}
document.querySelectorAll('[data-device-button]').forEach(btn=>{
 const name=btn.dataset.deviceButton;
 const labels={A:'A · Select',B:'B · Confirm',C:'C · Back'};
 btn.addEventListener('pointerdown',()=>btn.classList.add('pressed'));
 const release=()=>btn.classList.remove('pressed');
 btn.addEventListener('pointerup',release);btn.addEventListener('pointercancel',release);btn.addEventListener('pointerleave',release);
 btn.addEventListener('click',event=>{if(event.detail===0){reportDeviceInput('Button '+labels[name]+' pressed',{type:'button',button:name,pressed:true,source:'keyboard'});setTimeout(()=>reportDeviceInput('Button '+labels[name]+' released',{type:'button',button:name,pressed:false,source:'keyboard'}),80);}else reportDeviceInput('Button '+labels[name]+' pressed',{type:'button',button:name});});
});
const dial=$('zoomDial');let dialDragging=false,dialLastX=0,dialAngle=0,dialRemainder=0;
function rotateDial(delta,source='pointer'){
  dialAngle=(dialAngle+delta*3)%360;dial.style.setProperty('--dial-angle',dialAngle+'deg');dialRemainder+=delta;
  const steps=Math.trunc(dialRemainder/12);
  if(steps){dialRemainder-=steps*12;reportDeviceInput('Zoom dial rotated '+(steps>0?'forward':'backward')+' · '+Math.abs(steps)+' step(s)',{type:'dial',steps,source});}
 }
 dial.addEventListener('wheel',e=>{e.preventDefault();rotateDial(e.deltaY>0?1:-1,'wheel')},{passive:false});
 dial.addEventListener('pointerdown',e=>{dialDragging=true;dialLastX=e.clientX;dial.setPointerCapture(e.pointerId)});
dial.addEventListener('pointermove',e=>{if(!dialDragging)return;const dx=e.clientX-dialLastX;dialLastX=e.clientX;if(dx)rotateDial(dx)});
dial.addEventListener('pointerup',()=>dialDragging=false);dial.addEventListener('pointercancel',()=>dialDragging=false);
dial.addEventListener('click',()=>reportDeviceInput('Zoom dial pressed · Confirm',{type:'button',button:'DIAL'}));
dial.addEventListener('keydown',e=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();e.stopPropagation();reportDeviceInput('Zoom dial pressed · Confirm',{type:'button',button:'DIAL',pressed:true,source:'keyboard'});}else if(e.key==='ArrowRight'||e.key==='ArrowUp'){e.preventDefault();e.stopPropagation();rotateDial(12,'keyboard')}else if(e.key==='ArrowLeft'||e.key==='ArrowDown'){e.preventDefault();e.stopPropagation();rotateDial(-12,'keyboard')}});
 dial.addEventListener('keyup',e=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();e.stopPropagation();reportDeviceInput('Zoom dial released',{type:'button',button:'DIAL',pressed:false,source:'keyboard'});}});
const keyboardButtonMap={ArrowLeft:'C',ArrowRight:'A',Enter:'B',Escape:'C'};
 document.addEventListener('keydown',e=>{
  if(!$('view-emulator')?.classList.contains('active'))return;
  if(['INPUT','TEXTAREA','SELECT'].includes(document.activeElement?.tagName)||e.target===dial||e.target?.matches?.('[data-device-button]'))return;
  const button=keyboardButtonMap[e.key];
  if(button){e.preventDefault();reportDeviceInput('Keyboard → '+button+' button',{type:'button',button,pressed:true,source:'keyboard'});}
 });
 document.addEventListener('keyup',e=>{
  if(!$('view-emulator')?.classList.contains('active'))return;
  if(['INPUT','TEXTAREA','SELECT'].includes(document.activeElement?.tagName)||e.target===dial)return;
  const button=keyboardButtonMap[e.key];
  if(button)reportDeviceInput('Keyboard released '+button+' button',{type:'button',button,pressed:false,source:'keyboard'});
 });
 $('previewBg').oninput=renderPreview;$('previewScale').oninput=renderPreview;$('previewChecker').onchange=renderPreview;$('downloadPreviewBtn').onclick=()=>{const c=$('previewCanvas');c.toBlob(blob=>exportBlob(blob,'paradise-preview.png'))};$('emulatorLoadFirmwareBtn').onclick=()=>$('firmwareInput').click();
restoreWorkspace();
if(state.sprites.length){$('colorPicker').value=state.color;$('gridToggle').checked=state.grid;$('zoomSlider').value=String(state.zoom);$('zoomValue').textContent=state.zoom+'×';}
renderAll();

/* Experimental raw bitmap candidate scanner. Results are guesses, not decoded firmware assets. */
function scanRawCandidates(){
 const fw=state.firmware;if(!fw)return toast('Load firmware first');
 const bits=Number($('scanFormat').value),dim=$('scanSize').value;
 let w,h;if(dim.includes('x'))[w,h]=dim.split('x').map(Number);else w=h=Number(dim);
 const nbytes=Math.ceil(w*h*bits/8),data=fw.bytes,grid=$('candidateGrid'),found=[];
 grid.innerHTML='';
 for(let off=0;off+nbytes<=data.length&&found.length<80;off+=2){
  let min=255,max=0,sum=0;for(let i=0;i<nbytes;i++){const v=data[off+i];min=Math.min(min,v);max=Math.max(max,v);sum+=v}
  if(max-min<20)continue;const mean=sum/nbytes;let variance=0;
  for(let i=0;i<nbytes;i++)variance+=(data[off+i]-mean)**2;variance/=nbytes;
  if(variance<250||variance>14000)continue;
  found.push(off);
 }
 $('candidateCount').textContent=found.length;
 $('scanStatus').textContent=found.length?'Possible raw bitmap patterns only — not confirmed sprites. Compression, palettes and tile layouts are not decoded.':'No candidates matched this simple scan. The graphics may use compression or another format.';
 if(!found.length){grid.textContent='No candidates found for these settings.';return}
 for(const off of found){
  const card=document.createElement('article');card.className='candidate-card';
  const cv=document.createElement('canvas');cv.width=w;cv.height=h;
  const ctx=cv.getContext('2d'),im=ctx.createImageData(w,h);
  for(let p=0;p<w*h;p++){let v;if(bits===1)v=((data[off+(p>>3)]>>(7-(p&7)))&1)*255;else if(bits===2)v=((data[off+(p>>2)]>>(6-((p&3)*2)))&3)*85;else v=((data[off+(p>>1)]>>((p&1)?0:4))&15)*17;const k=p*4;im.data[k]=im.data[k+1]=im.data[k+2]=v;im.data[k+3]=255}
  ctx.putImageData(im,0,0);cv.className='candidate-art';cv.style.width=(w*4)+'px';cv.style.height=(h*4)+'px';
  const label=document.createElement('b');label.textContent=w+' × '+h;
  const meta=document.createElement('small');meta.textContent='Offset 0x'+off.toString(16).toUpperCase().padStart(6,'0');
  const btn=document.createElement('button');btn.className='button';btn.textContent='Export PNG';btn.onclick=()=>{const out=document.createElement('canvas');out.width=w;out.height=h;out.getContext('2d').drawImage(cv,0,0,w,h);out.toBlob(blob=>exportBlob(blob,'candidate_'+off.toString(16)+'.png'))};
  card.append(cv,label,meta,btn);grid.append(card);
 }
}
if($('scanFirmwareBtn'))$('scanFirmwareBtn').onclick=scanRawCandidates;


/* Documented Tamagotchi Paradise archive + sprite-package reader.
   Firmware remains in browser memory; this code never uploads it. */
const paradiseExtract = { packages: [], filtered: [], assets: [], filteredAssets: [], shown: 0, pageSize: 48 };
const readU16 = (a, p) => p >= 0 && p + 2 <= a.length ? a[p] | (a[p + 1] << 8) : 0;
const readU32 = (a, p) => p >= 0 && p + 4 <= a.length ? (a[p] | (a[p + 1] << 8) | (a[p + 2] << 16) | (a[p + 3] << 24)) >>> 0 : 0;
function parseSpriteHeader(data, start, packageLength) {
 if (start < 0 || start + 24 > packageLength) return null;
 const length=readU32(data,start), flags=data[start+4], bpp=data[start+5], count=readU16(data,start+6);
 const width=data[start+8], height=data[start+9], imageWidth=data[start+12], imageHeight=data[start+13];
 const paletteSets=data[start+15], transparent=readU16(data,start+16), paletteOffset=readU16(data,start+18), pixelOffset=readU16(data,start+20);
 const supported=[0,1,2,3,16,17,18].includes(bpp);
 if (!supported || !count || !width || !height || width>128 || height>128 || !imageWidth || !imageHeight) return null;
 if ((flags & 0x60) === 0x60) return null;
 if (length && (length<24 || start+length>packageLength)) return null;
 if (paletteOffset>= (length || packageLength-start) || pixelOffset>= (length || packageLength-start)) return null;
 return { length,flags,bpp,count,width,height,imageWidth,imageHeight,paletteSets,transparent,paletteOffset,pixelOffset };
}
function validateSpritePackage(data, absoluteOffset, length, sourceArchiveOffset, entryIndex) {
 if (length < 28 || absoluteOffset < 0 || absoluteOffset+length>data.length) return null;
 const first=readU32(data,absoluteOffset);
 if (first<4 || first%4!==0 || first>Math.min(length,0x20000)) return null;
 const count=first/4;
 if (count<1 || count>20000 || first+24>length) return null;
 let prev=first;
 for(let i=0;i<count;i++){
  const off=readU32(data,absoluteOffset+i*4);
  if(off<first || off<prev || off+24>length) return null;
  prev=off;
 }
 const firstHeader=parseSpriteHeader(data,absoluteOffset+readU32(data,absoluteOffset),absoluteOffset+length);
 if(!firstHeader) return null;
 let valid=0;
 for(let i=0;i<Math.min(count,16);i++){
  const h=parseSpriteHeader(data,absoluteOffset+readU32(data,absoluteOffset+i*4),absoluteOffset+length);
  if(h) valid++;
 }
 if(valid<Math.min(count,3)) return null;
 return { id:paradiseExtract.packages.length, offset:absoluteOffset, length, entryCount:count, sourceArchiveOffset, entryIndex, firstHeader };
}
function scanParadiseSpritePackages() {
 const fw=state.firmware;
 if(!fw) return toast('Load a firmware file first');
 const data=fw.bytes, packages=[], seen=new Set();
 const hasArc=(p)=>p>=0&&p+16<=data.length&&data[p]===0x41&&data[p+1]===0x52&&data[p+2]===0x43&&data[p+3]===0x32;
 for(let p=0;(p=data.indexOf(0x41,p))!==-1;p++){
  if(!hasArc(p)) continue;
  const total=readU32(data,p+8), files=readU32(data,p+12);
  if(!files||files>4096||total<files*16||p+16+total>data.length) continue;
  const table=p+16;
  if(table+files*16>p+16+total) continue;
  for(let i=0;i<files;i++){
   const e=table+i*16, off=readU32(data,e+4), comp=readU32(data,e+8), raw=readU32(data,e+12);
   if(off<16+files*16||!comp||off+Math.max(comp,raw)>16+total||p+off+Math.max(comp,raw)>data.length) continue;
   const absolute=p+off, entryLength=raw||comp, key=absolute+':'+entryLength;
   if(seen.has(key)) continue;
   const candidate=validateSpritePackage(data,absolute,entryLength,p,i);
   if(candidate){seen.add(key);packages.push(candidate)}
  }
 }
 // Deep pass: discover documented offset-table sprite packages outside recognised ARC2 archives.
 const known=new Set(packages.map(p=>p.offset));
 const archiveRanges=[];
 for(let p=0;(p=data.indexOf(0x41,p))!==-1;p++){
  if(p+16>data.length||data[p+1]!==0x52||data[p+2]!==0x43||data[p+3]!==0x32)continue;
  const total=readU32(data,p+8);
  if(total>0&&total<0x1000000&&p+16+total<=data.length)archiveRanges.push([p,p+16+total]);
 }
 let deepFound=0;
 for(let absolute=0;absolute+32<data.length;absolute+=4){
  if(known.has(absolute)||archiveRanges.some(r=>absolute>=r[0]&&absolute<r[1]))continue;
  const first=readU32(data,absolute);
  if(first<8||first%4!==0||first>0x4000||absolute+first+24>data.length)continue;
  const count=first/4;if(count<2||count>4096)continue;
  let prev=first,ok=true;
  for(let i=0;i<count;i++){const off=readU32(data,absolute+i*4);if(off<first||off<prev||absolute+off+24>data.length){ok=false;break}prev=off}
  if(!ok)continue;
  const firstHeader=parseSpriteHeader(data,absolute+readU32(data,absolute),data.length);
  if(!firstHeader||firstHeader.length<24||firstHeader.length>0x200000)continue;
  let plausible=0;
  for(let i=0;i<Math.min(count,8);i++){
   const off=readU32(data,absolute+i*4),hh=parseSpriteHeader(data,absolute+off,data.length);
   if(hh&&hh.length>=24&&hh.length<=0x200000&&absolute+off+hh.length<=data.length)plausible++;
  }
  if(plausible<Math.min(count,2))continue;
  const len=Math.min(data.length-absolute,0x800000);
  const candidate=validateSpritePackage(data,absolute,len,-1,-1);
  if(!candidate)continue;
  candidate.discovery='deep scan';known.add(absolute);packages.push(candidate);deepFound++;
 }
 packages.forEach(p=>{
  const h=p.firstHeader,w=h.width*h.imageWidth,ht=h.height*h.imageHeight;
  const frames=Math.floor(h.count/Math.max(1,h.imageWidth*h.imageHeight));
  // Labels are heuristic: the binary format does not contain human-readable asset names.
  p.category=(w>=96&&ht>=64)?'backgrounds':(w<=32&&ht<=32&&frames<=4)?'icons':(frames>=4&&(w>=24||ht>=24))?'characters':(w<=48&&ht<=48)?'ui':frames>=2?'effects':'other';
  if(!p.discovery)p.discovery='archive';
 });
 packages.sort((a,b)=>a.offset-b.offset);
 paradiseExtract.packages=packages;
 paradiseExtract.filtered=packages;
 paradiseExtract.shown=0;
 buildParadiseAssetIndex();
 const search=$('spritePackageSearch');
 if(search) search.disabled=!packages.length;
 $('spritePackageCount').textContent=paradiseExtract.assets.length+' assets · '+packages.length+' packages';
 $('spritePackageStatus').textContent=packages.length
  ? 'Decoded '+paradiseExtract.assets.length+' image entries from '+packages.length+' candidate sprite packages ('+deepFound+' deep-scan candidates). This is every valid sprite entry the current parser recognises; unsupported resource formats may still be missing.'
  : 'No documented sprite packages found in this file. Confirm that this is a complete Paradise flash dump.';
 renderSpritePackagePage(true);
}
function rleDecode(src, wordwise, maxOutput) {
 const out=[]; let p=0; const unit=wordwise?4:1;
 while(p+unit<=src.length && out.length<maxOutput){
  let ctrl=0; for(let j=0;j<unit;j++) ctrl=(ctrl | (src[p+j]<<(j*8)))>>>0; p+=unit;
  if(ctrl===0) break;
  const literal=wordwise ? !!(ctrl&0x80000000) : !!(ctrl&0x80);
  const n=wordwise ? (ctrl&0x0fffffff) : (ctrl&0x7f);
  if(literal){
   const bytes=Math.min(n*unit,src.length-p,maxOutput-out.length);
   for(let j=0;j<bytes;j++) out.push(src[p+j]);
   p+=bytes;
  }else{
   if(p+unit>src.length) break;
   const value=Array.from(src.subarray(p,p+unit));p+=unit;
   const bytes=Math.min(n*unit,maxOutput-out.length);
   for(let j=0;j<bytes;j++) out.push(value[j%unit]);
  }
 }
 return new Uint8Array(out);
}
function decodeParadiseSpriteFrame(pkg, assetIndex, frameIndex, paletteSet=0) {
 const fw=state.firmware;if(!fw) throw new Error('No firmware loaded');
 const all=fw.bytes, base=pkg.offset, rel=readU32(all,base+assetIndex*4), start=base+rel;
 const h=parseSpriteHeader(all,start,base+pkg.length);
 if(!h) throw new Error('Invalid sprite header');
 if(frameIndex<0||frameIndex>=h.count) throw new Error('Frame index is outside this sprite package');
 const pixelsPerFrame=h.width*h.height;
 const indexed=h.bpp<=3, bits=indexed?[1,2,4,8][h.bpp]:(h.bpp===18?24:16);
 const expected=Math.ceil(pixelsPerFrame*bits/8);
 let decoded;
 const pixelBase=start+h.pixelOffset;
 if(h.flags&0x60){
  const relOffset=readU32(all,pixelBase+frameIndex*8), storedLength=readU32(all,pixelBase+frameIndex*8+4);
  const rawOffset=relOffset&0x7fffffff, absolute=pixelBase+rawOffset;
  if(absolute<0||storedLength>pkg.length||absolute+storedLength>base+pkg.length||(h.length&&absolute+storedLength>start+h.length)) throw new Error('Pixel data range is invalid');
  let stored=all.slice(absolute,absolute+storedLength);
  if(h.flags&0x80) stored=stored.map(v=>v^0x53);
  decoded=(relOffset&0x80000000)?stored:rleDecode(stored,!!(h.flags&0x40),Math.min(expected*4+64,2*1024*1024));
 }else{
  const absolute=pixelBase+frameIndex*expected;
  if(absolute+expected>base+pkg.length||(h.length&&absolute+expected>start+h.length)) throw new Error('Uncompressed pixel data is truncated');
  decoded=all.slice(absolute,absolute+expected);
  if(h.flags&0x80) decoded=decoded.map(v=>v^0x53);
 }
 const rgba=new Uint8ClampedArray(pixelsPerFrame*4), colors=[];
 if(indexed){
  const maxColors=1<<bits, paletteBase=start+h.paletteOffset+Math.min(paletteSet,Math.max(0,h.paletteSets-1))*maxColors*2;
  for(let i=0;i<maxColors;i++){
   const c=readU16(all,paletteBase+i*2), r=((c>>11)&31)*255/31, g=((c>>5)&63)*255/63, b=(c&31)*255/31;
   colors.push('#'+[r,g,b].map(v=>Math.round(v).toString(16).padStart(2,'0')).join(''));
  }
  let bit=0;
  for(let i=0;i<pixelsPerFrame;i++){
   const byte=bit>>3, shift=bit&7;
   let idx=0;
   for(let k=0;k<bits;k++) if(byte+(shift+k>>3)<decoded.length) idx|=((decoded[(bit+k)>>3]>>((bit+k)&7))&1)<<k;
   const c=readU16(all,paletteBase+idx*2), r=Math.round(((c>>11)&31)*255/31),g=Math.round(((c>>5)&63)*255/63),b=Math.round((c&31)*255/31);
   const at=i*4;rgba[at]=r;rgba[at+1]=g;rgba[at+2]=b;rgba[at+3]=((h.flags&4)&&idx===h.transparent)?0:255;bit+=bits;
  }
 }else{
  for(let i=0;i<pixelsPerFrame;i++){
   let r,g,b,alpha=255,word=0;
   if(h.bpp===18){const k=i*3;r=decoded[k]||0;g=decoded[k+1]||0;b=decoded[k+2]||0;word=((r>>3)<<11)|((g>>2)<<5)|(b>>3)}
   else{const k=i*2;word=readU16(decoded,k);r=Math.round(((word>>11)&31)*255/31);g=Math.round(((word>>5)&63)*255/63);b=Math.round((word&31)*255/31)}
   if((h.flags&4)&&word===h.transparent)alpha=0;
   const at=i*4;rgba[at]=r;rgba[at+1]=g;rgba[at+2]=b;rgba[at+3]=alpha;
  }
 }
 return {width:h.width,height:h.height,rgba,header:h};
}
function decodeParadiseSubimage(pkg, assetIndex, subimageIndex, paletteSet=0) {
 const fw=state.firmware, all=fw.bytes, base=pkg.offset, rel=readU32(all,base+assetIndex*4), start=base+rel;
 const h=parseSpriteHeader(all,start,base+pkg.length);if(!h)throw new Error('Invalid sprite header');
 const cols=Math.max(1,h.imageWidth), rows=Math.max(1,h.imageHeight), per=cols*rows;
 const total=Math.floor(h.count/per), index=Math.max(0,Math.min(total-1,subimageIndex));
 const tileW=h.width,tileH=h.height,rgba=new Uint8ClampedArray(tileW*cols*tileH*rows*4);
 for(let ty=0;ty<rows;ty++)for(let tx=0;tx<cols;tx++){
  const tile=decodeParadiseSpriteFrame(pkg,assetIndex,index*per+ty*cols+tx,paletteSet);
  for(let y=0;y<tileH;y++){
   const src=(y*tileW)*4,dst=((ty*tileH+y)*(tileW*cols)+tx*tileW)*4;
   rgba.set(tile.rgba.subarray(src,src+tileW*4),dst);
  }
 }
 return {width:tileW*cols,height:tileH*rows,rgba,header:h};
}
function rgbaToPixels(decoded) {
 const pixels=[], palette=[];
 const map=new Map();
 for(let i=0;i<decoded.rgba.length;i+=4){
  if(decoded.rgba[i+3]===0){pixels.push(null);continue}
  const color='#'+[decoded.rgba[i],decoded.rgba[i+1],decoded.rgba[i+2]].map(v=>v.toString(16).padStart(2,'0')).join('');
  pixels.push(color);if(!map.has(color)){map.set(color,true);palette.push(color)}
 }
 return {pixels,palette:palette.length?palette:['#000000']};
}
function drawDecodedSprite(canvas, decoded) {
 canvas.width=decoded.width;canvas.height=decoded.height;
 const ctx=canvas.getContext('2d');ctx.imageSmoothingEnabled=false;
 ctx.putImageData(new ImageData(decoded.rgba,decoded.width,decoded.height),0,0);
 canvas.style.width=Math.max(64,Math.min(160,decoded.width*3))+'px';
 canvas.style.height=Math.max(64,Math.min(160,decoded.height*3))+'px';
}
function buildParadiseAssetIndex() {
 const data=state.firmware?.bytes;if(!data)return;
 const assets=[];
 for(const pkg of paradiseExtract.packages){
  for(let ai=0;ai<pkg.entryCount;ai++){
   const rel=readU32(data,pkg.offset+ai*4),absolute=pkg.offset+rel;
   const h=parseSpriteHeader(data,absolute,pkg.offset+pkg.length);
   if(!h)continue;
   const subimages=Math.floor(h.count/(h.imageWidth*h.imageHeight));
   if(!subimages)continue;
   const w=h.width*h.imageWidth,ht=h.height*h.imageHeight;
   const category=(w>=96&&ht>=64)?'backgrounds':(w<=32&&ht<=32&&subimages<=4)?'icons':(subimages>=4&&(w>=24||ht>=24))?'characters':(w<=48&&ht<=48)?'ui':subimages>=2?'effects':'other';
   assets.push({pkg,assetIndex:ai,header:h,subimages,width:w,height:ht,category,id:pkg.offset+':'+ai});
  }
 }
 paradiseExtract.assets=assets;
}
function renderSpritePackagePage(reset=false) {
 const grid=$('spritePackageResults');if(!grid)return;
 if(reset){grid.innerHTML='';paradiseExtract.shown=0}
 const search=($('spritePackageSearch')?.value||'').trim().toLowerCase();
 const category=$('spriteCategoryFilter')?.value||'all';
 paradiseExtract.filteredAssets=paradiseExtract.assets.filter(a=>{
  const h=a.header,p=a.pkg;
  const hay=[a.assetIndex,p.offset.toString(16),p.offset.toString(10),h.width+'x'+h.height,a.width+'x'+a.height,h.bpp,h.count,a.subimages,a.category,p.discovery].join(' ').toLowerCase();
  return (category==='all'||a.category===category)&&hay.includes(search);
 });
 if(reset)paradiseExtract.shown=0;
 const start=paradiseExtract.shown,end=Math.min(start+paradiseExtract.pageSize,paradiseExtract.filteredAssets.length);
 for(let n=start;n<end;n++){
  const asset=paradiseExtract.filteredAssets[n],pkg=asset.pkg,h=asset.header,ai=asset.assetIndex;
  const card=document.createElement('article');card.className='sprite-package-card asset-browser-card';
  const canvas=document.createElement('canvas');canvas.className='sprite-package-preview';
  const title=document.createElement('b');title.textContent=asset.category.toUpperCase()+' · Asset '+ai+' · 0x'+pkg.offset.toString(16).toUpperCase();
  const meta=document.createElement('small');meta.textContent=asset.width+'×'+asset.height+' composed · '+asset.subimages+' subimage(s) · '+h.width+'×'+h.height+' tile · '+(h.bpp<=3?[1,2,4,8][h.bpp]+'bpp indexed':(h.bpp===18?'RGB888':'RGB565'))+' · '+pkg.discovery;
  const frameRow=document.createElement('label');frameRow.className='sprite-frame-row';frameRow.textContent='Subimage ';
  const frameInput=document.createElement('input');frameInput.type='number';frameInput.min='0';frameInput.max=String(asset.subimages-1);frameInput.value='0';frameInput.setAttribute('aria-label','Subimage index');
  frameRow.append(frameInput,document.createTextNode(' / '+asset.subimages));
  const paletteRow=document.createElement('label');paletteRow.className='sprite-frame-row';paletteRow.textContent='Palette ';
  const paletteInput=document.createElement('input');paletteInput.type='number';paletteInput.min='0';paletteInput.max=String(Math.max(0,h.paletteSets-1));paletteInput.value='0';paletteInput.setAttribute('aria-label','Palette set index');
  paletteRow.append(paletteInput,document.createTextNode(' / '+Math.max(1,h.paletteSets)));
  const refresh=()=>{try{const fi=Math.max(0,Math.min(asset.subimages-1,Number(frameInput.value)||0)),pi=Math.max(0,Math.min(Math.max(0,h.paletteSets-1),Number(paletteInput.value)||0));frameInput.value=String(fi);paletteInput.value=String(pi);drawDecodedSprite(canvas,decodeParadiseSubimage(pkg,ai,fi,pi))}catch(err){canvas.width=32;canvas.height=32;const ctx=canvas.getContext('2d');ctx.clearRect(0,0,32,32);const label=document.createElement('small');label.textContent='Decode error';card.dataset.error=err.message}};
  frameInput.addEventListener('change',refresh);paletteInput.addEventListener('change',refresh);
  const importBtn=document.createElement('button');importBtn.className='button primary';importBtn.textContent='Import subimage';
  importBtn.onclick=()=>{try{const fi=Math.max(0,Math.min(asset.subimages-1,Number(frameInput.value)||0)),pi=Math.max(0,Math.min(Math.max(0,h.paletteSets-1),Number(paletteInput.value)||0)),d=decodeParadiseSubimage(pkg,ai,fi,pi),converted=rgbaToPixels(d);createSprite('Water asset '+ai+' subimage '+fi,d.width,d.height,converted.pixels,converted.palette);nav('studio');toast('Composed subimage imported')}catch(err){toast('Import failed: '+err.message)}};
  const exportBtn=document.createElement('button');exportBtn.className='button';exportBtn.textContent='Export composed PNG';
  exportBtn.onclick=()=>{try{const fi=Math.max(0,Math.min(asset.subimages-1,Number(frameInput.value)||0)),pi=Math.max(0,Math.min(Math.max(0,h.paletteSets-1),Number(paletteInput.value)||0)),d=decodeParadiseSubimage(pkg,ai,fi,pi),cv=document.createElement('canvas');cv.width=d.width;cv.height=d.height;cv.getContext('2d').putImageData(new ImageData(d.rgba,d.width,d.height),0,0);cv.toBlob(blob=>exportBlob(blob,'paradise_asset_'+ai+'_subimage_'+fi+'.png'))}catch(err){toast('Export failed: '+err.message)}};
  card.append(canvas,title,meta,frameRow,paletteRow,importBtn,exportBtn);grid.append(card);refresh();
 }
 paradiseExtract.shown=end;
 const more=$('spritePackageMore');more.hidden=end>=paradiseExtract.filteredAssets.length;
 more.textContent='Show more assets ('+end+' of '+paradiseExtract.filteredAssets.length+')';
 $('spritePackageCount').textContent=paradiseExtract.assets.length+' assets · '+paradiseExtract.packages.length+' packages';
 if(!paradiseExtract.filteredAssets.length)grid.textContent='No decoded assets match this filter. Try All asset types or scan a full firmware dump.';
}
if($('findSpritePackagesBtn')) $('findSpritePackagesBtn').addEventListener('click',scanParadiseSpritePackages);
if($('spritePackageSearch')) $('spritePackageSearch').addEventListener('input',()=>renderSpritePackagePage(true));
if($('spriteCategoryFilter')) $('spriteCategoryFilter').addEventListener('change',()=>renderSpritePackagePage(true));
if($('spritePackageMore')) $('spritePackageMore').addEventListener('click',()=>renderSpritePackagePage(false));


/* Lightweight frame-by-frame animation timeline */
function cloneAnimationFrame(s) { return {name:s.name,width:s.width,height:s.height,pixels:[...s.pixels],palette:[...s.palette]}; }
function updateAnimationCount() { $('animationFrameCount').textContent=animationState.frames.length+' frame'+(animationState.frames.length===1?'':'s'); }
function stopAnimation() { if(animationState.timer){clearInterval(animationState.timer);animationState.timer=null} const b=$('animationPlay');if(b)b.textContent='▶ Play'; }
function drawAnimationFrame(frame,canvas) {
 const ctx=canvas.getContext('2d');canvas.width=frame.width;canvas.height=frame.height;const im=ctx.createImageData(frame.width,frame.height);
 for(let i=0;i<frame.pixels.length;i++){const color=frame.pixels[i],k=i*4;if(!color){im.data[k+3]=0;continue}const hex=color.replace('#','');im.data[k]=parseInt(hex.slice(0,2),16)||0;im.data[k+1]=parseInt(hex.slice(2,4),16)||0;im.data[k+2]=parseInt(hex.slice(4,6),16)||0;im.data[k+3]=255}
 ctx.putImageData(im,0,0);
}
function renderAnimationStrip() {
 const strip=$('animationFrameStrip');strip.innerHTML='';
 if(!animationState.frames.length){strip.innerHTML='<div class="empty-state">No animation frames yet. Add your current sprite to begin.</div>';updateAnimationCount();return}
 animationState.frames.forEach((frame,i)=>{const card=document.createElement('button');card.type='button';card.className='animation-frame'+(i===animationState.selected?' selected':'');card.title='Select frame '+(i+1);
 const cv=document.createElement('canvas');cv.width=frame.width;cv.height=frame.height;drawAnimationFrame(frame,cv);const thumb=document.createElement('span');thumb.className='animation-thumb';thumb.append(cv);
 const label=document.createElement('b');label.textContent='Frame '+(i+1);const dims=document.createElement('small');dims.textContent=frame.width+'×'+frame.height;card.append(thumb,label,dims);
 card.addEventListener('click',()=>{stopAnimation();animationState.selected=i;loadAnimationFrame(i);renderAnimationStrip()});strip.append(card)});
 updateAnimationCount();
}
function loadAnimationFrame(index) {
 const frame=animationState.frames[index];if(!frame)return;let s=active();
 if(!s){createSprite(frame.name,frame.width,frame.height,[...frame.pixels],[...frame.palette]);return}
 snap();s.name=frame.name||'Animation Frame '+(index+1);s.width=frame.width;s.height=frame.height;s.pixels=[...frame.pixels];s.palette=[...frame.palette];
 $('spriteName').value=s.name;$('spriteWidth').value=s.width;$('spriteHeight').value=s.height;renderAll();$('animationPreviewLabel').textContent='Frame '+(index+1)+' of '+animationState.frames.length;drawAnimationFrame(frame,$('animationPreviewCanvas'));
}
function addAnimationFrame() {
 const s=active();if(!s)return toast('Create or select a sprite first');animationState.frames.push(cloneAnimationFrame(s));animationState.selected=animationState.frames.length-1;stopAnimation();renderAnimationStrip();drawAnimationFrame(animationState.frames[animationState.selected],$('animationPreviewCanvas'));$('animationPreviewLabel').textContent='Frame '+(animationState.selected+1)+' ready';
}
function updateAnimationFrame() {
 const s=active();if(!s)return toast('Create or select a sprite first');if(animationState.selected<0||!animationState.frames[animationState.selected])return toast('Select or add an animation frame first');
 animationState.frames[animationState.selected]=cloneAnimationFrame(s);renderAnimationStrip();drawAnimationFrame(animationState.frames[animationState.selected],$('animationPreviewCanvas'));toast('Animation frame updated');
}
function playAnimation() {
 if(animationState.frames.length<2)return toast('Add at least two frames to play an animation');if(animationState.timer){stopAnimation();return}
 const fps=Math.max(1,Number($('animationFps').value)||4);let i=animationState.selected>=0?animationState.selected:0;$('animationPlay').textContent='■ Stop';
 const tick=()=>{const frame=animationState.frames[i];drawAnimationFrame(frame,$('animationPreviewCanvas'));$('animationPreviewLabel').textContent='Playing frame '+(i+1)+' / '+animationState.frames.length;animationState.selected=i;renderAnimationStrip();i=(i+1)%animationState.frames.length};
 tick();animationState.timer=setInterval(tick,1000/fps);
}
function exportAnimationJson() {
 if(!animationState.frames.length)return toast('Add at least one animation frame first');
 const payload={format:'paradise-mod-studio-animation',version:1,fps:Number($('animationFps').value)||4,frames:animationState.frames.map(f=>({name:f.name,width:f.width,height:f.height,pixels:f.pixels,palette:f.palette}))};
 exportBlob(new Blob([JSON.stringify(payload,null,2)],{type:'application/json'}),'paradise-animation.json');
}
$('animationAddFrame').addEventListener('click',addAnimationFrame);
$('animationUpdateFrame').addEventListener('click',updateAnimationFrame);
$('animationDeleteFrame').addEventListener('click',()=>{if(animationState.selected<0||!animationState.frames.length)return toast('Select a frame to delete first');stopAnimation();animationState.frames.splice(animationState.selected,1);animationState.selected=animationState.frames.length?Math.min(animationState.selected,animationState.frames.length-1):-1;renderAnimationStrip();if(animationState.selected>=0){loadAnimationFrame(animationState.selected)}else{$('animationPreviewLabel').textContent='Preview idle';const ctx=$('animationPreviewCanvas').getContext('2d');ctx.clearRect(0,0,$('animationPreviewCanvas').width,$('animationPreviewCanvas').height)}toast('Animation frame deleted')});
$('animationPrevFrame').addEventListener('click',()=>{if(!animationState.frames.length)return;stopAnimation();animationState.selected=(animationState.selected-1+animationState.frames.length)%animationState.frames.length;loadAnimationFrame(animationState.selected);renderAnimationStrip()});
$('animationNextFrame').addEventListener('click',()=>{if(!animationState.frames.length)return;stopAnimation();animationState.selected=(animationState.selected+1)%animationState.frames.length;loadAnimationFrame(animationState.selected);renderAnimationStrip()});
$('animationPlay').addEventListener('click',playAnimation);
$('animationExport').addEventListener('click',exportAnimationJson);
$('animationFps').addEventListener('input',e=>{$('animationFpsValue').textContent=e.target.value;if(animationState.timer){stopAnimation();playAnimation()}});
