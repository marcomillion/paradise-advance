'use strict';
// Firmware frontend only. This intentionally does not implement substitute
// pet gameplay: the screen must be driven by the actual SNC73410 core.
(function(){
 const canvas=document.getElementById('previewCanvas');
 const empty=document.getElementById('previewEmpty');
 if(!canvas||!empty)return;
 const title=document.getElementById('emulatorStatusTitle');
 const status=document.getElementById('emulatorStatusText');
 const input=document.getElementById('firmwareInput');
 const button=document.getElementById('emulatorLoadFirmwareBtn');
 const statusLine=document.getElementById('deviceInputStatus');
 let firmware=null;
 function message(t){if(status)status.textContent=t;}
 function unavailable(){
   empty.style.display='grid';
   empty.innerHTML='REAL FIRMWARE<br>CORE NOT BUILT';
   if(title)title.textContent='WebAssembly core not integrated';
   message('No substitute pet is running. The browser build still needs the real SNC73410 CPU, encrypted-flash loader, and display/input peripherals compiled to WebAssembly.');
 }
 async function inspect(file){
   if(!file)return;
   try{
     const bytes=new Uint8Array(await file.arrayBuffer());
     firmware=bytes;
     const magic=new TextDecoder().decode(bytes.subarray(0,8));
     const valid=magic==='SONIXDEV'&&bytes.length===16777216;
     if(title)title.textContent=valid?'Paradise firmware dump recognised':'Unrecognised firmware image';
     message(valid
       ? 'Loaded locally: '+file.name+' ('+bytes.length.toLocaleString()+' bytes). SONIXDEV header and 16 MiB size confirmed. The real game is not executing yet: the WebAssembly core still needs to be ported.'
       : 'Selected '+file.name+' ('+bytes.length.toLocaleString()+' bytes). Expected a 16 MiB SONIXDEV dump. No firmware will be executed until the real core is integrated.');
     empty.style.display='grid';
     empty.innerHTML=valid?'FIRMWARE LOADED<br>CPU CORE NOT CONNECTED':'CHECK FIRMWARE FILE';
   }catch(err){message('Could not read the firmware file: '+err.message);}
 }
 if(button&&input)button.addEventListener('click',()=>input.click());
 if(input)input.addEventListener('change',()=>inspect(input.files&&input.files[0]));
 document.addEventListener('paradise:device-input',e=>{
   const d=e.detail||{};
   if(statusLine)statusLine.textContent='Input received ('+(d.button||d.type||'unknown')+') · core not connected';
 });
 unavailable();
 window.paradiseFirmwareFrontend={inspect,get firmware(){return firmware}};
})();