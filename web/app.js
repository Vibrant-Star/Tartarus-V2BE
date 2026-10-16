'use strict';
const $ = id => document.getElementById(id);
const token = document.querySelector('meta[name="editor-token"]').content;
let pollTimer, closed = false;
let macroDraft=[];
let draft, savedText = '', savedCanonical = '', selected = '01', layer = 0, profiles = [], busy = false, recording = null, timer, lastStatus;
const modifiers = ['LeftCtrl','RightCtrl','LeftAlt','RightAlt','LeftShift','RightShift','LeftWin','RightWin'];
const controlName = name => /^\d+$/.test(name) ? `Key ${name}` : ({thumb:'Thumb button',wheel_up:'Wheel up',wheel_down:'Wheel down',wheel_click:'Wheel click',dpad_up:'D-pad ↑',dpad_down:'D-pad ↓',dpad_left:'D-pad ←',dpad_right:'D-pad →'}[name] || name);
const canonical = value => JSON.stringify(value);
const text = () => JSON.stringify(draft, null, 2) + '\n';
const dirty = () => draft && canonical(draft) !== savedCanonical;
function notice(message, error = false) { $('notice').textContent = message; $('notice').classList.toggle('error', error); }
async function api(path, body) {
  const response = await fetch(`/api/${path}`, {method:body===undefined?'GET':'POST',headers:{'X-Tartarus-Token':token,...(body===undefined?{}:{'Content-Type':'application/json'})},...(body===undefined?{}:{body:JSON.stringify(body)})});
  const result = await response.json();
  if (!response.ok) throw Error(result.error || 'The editor could not complete this request.');
  return result;
}
function actionLabel(action) { if(!action || action.type==='passthrough') return 'Native'; if(action.type==='macro')return 'Macro · '+action.steps.length+' steps'; if(action.type==='disabled') return 'Disabled'; if(action.type==='mouse_button')return `Mouse ${action.button}`;if(action.type==='mouse_wheel')return `Wheel ${action.direction}`;return action.type==='chord'?action.keys.join(' + '):action.key; }
function maps() { return layer===0 ? (draft.mappings ||= {}) : (draft.layers[layer-1].mappings ||= {}); }
function layerNames() { return ['Base',...(draft.layers||[]).map(l=>l.name)]; }
function option(value,label) { const o=document.createElement('option');o.value=value;o.textContent=label;return o; }
function control(name, label) { const b=document.createElement('button');b.className='control';b.dataset.control=name;const s=document.createElement('span');s.textContent=label;const v=document.createElement('strong');b.append(s,v);b.addEventListener('click',()=>{selected=name;renderControls();renderInspector();});return b; }
for(let n=1;n<=19;n++){const name=String(n).padStart(2,'0');$('keypad').append(control(name,name));}
const wheel = document.createElement('div');wheel.className='wheel';wheel.id='wheel';$('keypad').append(wheel);$('spaceControl').append(control('20','20'));
for(const [name,label] of [['wheel_up','SCROLL ↑'],['wheel_click','WHEEL CLICK'],['wheel_down','SCROLL ↓']]) $('wheel').append(control(name,label));
for(const [name,label] of [['dpad_up','↑'],['dpad_left','←'],['dpad_right','→'],['dpad_down','↓']]) $('dpad').append(control(name,label));
$('thumb').addEventListener('click',()=>{selected='thumb';renderControls();renderInspector();});
function renderControls(){
  const bindings=maps();
  document.querySelectorAll('[data-control]').forEach(b=>{
    const name=b.dataset.control;
    let label=actionLabel(bindings[name]);
    if(name==='thumb'&&draft.thumb) label=draft.thumb.mode==='modifier'?draft.thumb.key||'LeftAlt':draft.thumb.mode==='momentary'?`Hold → ${draft.thumb.layer}`:'Cycle layers';
    b.querySelector('strong').textContent=label;b.classList.toggle('selected',name===selected);b.setAttribute('aria-pressed',String(name===selected));b.setAttribute('aria-label',`${controlName(name)}: ${label}`);b.title=`${controlName(name)}: ${label}`;
  });
  $('apply').textContent=dirty()?'Apply changes •':'Apply changes';
  if(lastStatus) paintStatus(lastStatus);
}
function renderInspector(){
  $('controlTitle').textContent=controlName(selected);$('layerCaption').textContent=layerNames()[layer];
  const modeControlled=selected==='thumb'&&!!draft.thumb;
  const a=maps()[selected]||{type:'passthrough'};$('action').value=a.type;$('keys').value=a.type==='chord'?a.keys.join(' + '):a.type==='key'?a.key:'';
  $('mouseButton').value=a.type==='mouse_button'?a.button:'left';$('wheelDirection').value=a.type==='mouse_wheel'?a.direction:'up';
  for(const id of ['action','keys','mouseButton','wheelDirection','assign','record']) $(id).disabled=modeControlled;
  $('assign').textContent=modeControlled?'Use Thumb behavior settings':'Update assignment';
  macroDraft=structuredClone(a.type==='macro'?a.steps:[{type:'tap',keys:['A'],hold_ms:40},{type:'delay',ms:100},{type:'tap',keys:['B'],hold_ms:40}]);renderMacro();
  actionFields();
}
function actionFields(){const type=$('action').value;const needs=['key','chord'].includes(type);$('keysLabel').hidden=!needs;$('record').hidden=!needs;$('recordHint').hidden=!needs;$('mouseButtonLabel').hidden=type!=='mouse_button';$('wheelDirectionLabel').hidden=type!=='mouse_wheel';$('macroEditor').hidden=type!=='macro';}
function renderThumb(){
  const thumb=draft.thumb;$('thumbMode').value=thumb?.mode||'legacy';const value=$('thumbValue');value.replaceChildren();
  const mode=$('thumbMode').value;$('thumbValueLabel').hidden=mode==='legacy'||mode==='cycle';
  if(mode==='modifier'){ $('thumbValueLabel').firstChild.textContent='Modifier'; modifiers.forEach(k=>value.append(option(k,k)));value.value=thumb.key||'LeftAlt'; }
  if(mode==='momentary'){ $('thumbValueLabel').firstChild.textContent='Target layer';layerNames().slice(1).forEach((n,i)=>value.append(option(i+1,n)));value.value=String(thumb.layer); }
  $('thumbHelp').textContent=mode==='cycle'?'Cycles through every configured layer in order.':mode==='momentary'?'Hold the thumb to use the target layer. Release to return.':mode==='modifier'?'The thumb sends this modifier, independent of the layer.':'Assign the thumb separately on each layer.';
}
function render(){
  const select=$('layer');select.replaceChildren();layerNames().forEach((name,i)=>select.append(option(i,name)));select.value=String(layer);$('addLayer').disabled=(draft.layers||[]).length>=2;
  $('layerName').disabled=layer===0;$('layerName').value=layerNames()[layer];renderThumb();renderControls();renderInspector();
}
async function load(){const result=await api('config');savedText=result.text;draft=JSON.parse(savedText.replace(/^\uFEFF/,''));savedCanonical=canonical(draft);layer=0;$('profiles').value='';$('profileName').value='Current-configuration';render();notice('Saved configuration loaded. Select any control to edit.');}
async function library(){profiles=(await api('profiles')).profiles;const s=$('profiles');const previous=s.value;s.replaceChildren(option('','Current configuration'));profiles.sort((a,b)=>a.name.localeCompare(b.name)).forEach(p=>s.append(option(p.name,p.name)));s.value=previous;}
async function status(){try{lastStatus=await api('status');paintStatus(lastStatus);}catch(e){$('status').textContent='Editor unavailable';$('start').disabled=true;$('stop').disabled=true;} }
function paintStatus(s){
  paintRgbStatus(s.lighting);
  $('status').textContent=s.running?(s.applied?'Remapping active · settings applied':'Remapping active · settings pending'):s.recovering?`Waiting to reconnect · ${s.recovering}`:s.starting?'Starting remapping…':s.error?`Stopped · ${s.error}`:'Remapping stopped';
  $('start').disabled=!!(s.running||s.starting||dirty());$('start').title=dirty()?'Apply your draft before starting':'';$('stop').disabled=!(s.running||s.starting);$('device').textContent=s.device;$('driver').textContent=s.driver;
  $('activeLayer').textContent=liveFrame?.running?`Active: ${liveFrame.layer_names[liveFrame.layer]||liveFrame.layer}`:s.running?`Active layer: ${s.layer+1}`:'Runtime stopped';
}
function listen(id,event,fn){$(id).addEventListener(event,async ev=>{try{await fn(ev);}catch(e){notice(e.message,true);}});}
listen('layer','change',()=>{layer=Number($('layer').value);render();});
listen('layerName','change',()=>{if(layer){draft.layers[layer-1].name=$('layerName').value.trim();render();notice('Layer renamed in draft. Apply when ready.');}});
listen('addLayer','click',()=>{draft.layers||=[];if(draft.layers.length>=2)return;let name=`Layer ${draft.layers.length+1}`;while(layerNames().includes(name))name+=' new';draft.layers.push({name,mappings:{}});layer=draft.layers.length;if(draft.thumb?.mode==='cycle')draft.thumb.layers=layerNames().map((_,i)=>i);render();notice('Independent layer added. Unassigned controls retain their native input.');});
listen('action','change',actionFields);
listen('assign','click',async()=>{
  const originalDraft=draft, originalLayer=layer, originalSelected=selected;
  const type=$('action').value;let action={type};
  if(type==='mouse_button')action.button=$('mouseButton').value;
  if(type==='mouse_wheel')action.direction=$('wheelDirection').value;
  if(type==='key'){const key=$('keys').value.trim();if(!key)throw Error('Enter a key first.');action.key=key;}
  if(type==='chord'){action.keys=$('keys').value.split('+').map(s=>s.trim()).filter(Boolean);if(!action.keys.length)throw Error('Enter or record a combination first.');}
  if(type==='macro'){action.steps=structuredClone(macroDraft);if(!action.steps.length)throw Error('Add at least one macro step.');const candidate=structuredClone(draft);const bindings=layer===0?(candidate.mappings||={}):(candidate.layers[layer-1].mappings||={});bindings[selected]=action;await api('validate',{text:JSON.stringify(candidate)});}
  if(draft!==originalDraft||layer!==originalLayer||selected!==originalSelected)throw Error('Selection changed during validation. Update the assignment again.');
  maps()[selected]=action;renderControls();notice(`${controlName(selected)} updated in the draft. Apply changes to use it.`);
});
listen('thumbMode','change',()=>{
  const mode=$('thumbMode').value;
  if(['momentary','cycle'].includes(mode)&&!draft.layers?.length){$('thumbMode').value=draft.thumb?.mode||'legacy';throw Error('Add a second layer first.');}
  if(mode==='legacy')delete draft.thumb;
  else{delete draft.mappings?.thumb;for(const l of draft.layers||[])delete l.mappings?.thumb;draft.thumb=mode==='modifier'?{mode,key:'LeftAlt'}:mode==='momentary'?{mode,layer:1}:{mode,layers:layerNames().map((_,i)=>i)};}
  render();notice('Thumb mode updated in the draft. Per-layer thumb assignments are cleared when selecting a shared mode.');
});
listen('thumbValue','change',()=>{if(draft.thumb.mode==='modifier')draft.thumb.key=$('thumbValue').value;else draft.thumb.layer=Number($('thumbValue').value);renderControls();});
listen('apply','click',async()=>{
  if(busy)return;busy=true;$('apply').disabled=true;
  try{const next=text();await api('config',{expected:savedText,text:next});savedText=next;savedCanonical=canonical(draft);renderControls();notice('Configuration saved. An active runtime applies it without restarting; check the status above.');await status();}finally{busy=false;$('apply').disabled=false;}
});
listen('reload','click',async()=>{if(!dirty()||confirm('Discard the current draft and reload the saved configuration?'))await load();});
listen('start','click',async()=>{await api('start',{seconds:duration.value?Number(duration.value):null});notice('Starting remapping with the saved configuration. Release all keyboard keys and mouse middle buttons.');await status();});
listen('stop','click',async()=>{await api('stop',{});notice('Stop requested. Wait for “Remapping stopped” before disconnecting.');await status();});
listen('profiles','change',async()=>{const name=$('profiles').value;if(dirty()&&!confirm('Discard the current draft?')){$('profiles').value='';return;}if(!name){await load();return;}const p=profiles.find(p=>p.name===name);draft=JSON.parse(p.text.replace(/^\uFEFF/,''));$('profileName').value=p.name;layer=0;render();notice(`Loaded ${name} as a draft. Apply changes to activate it.`);});
listen('new','click',()=>{if(dirty()&&!confirm('Discard the current draft?'))return;draft={schema_version:1,mappings:{}};layer=0;$('profiles').value='';$('profileName').value='New-profile';render();notice('New profile draft. All controls currently use native input.');});
listen('duplicate','click',()=>{$('profiles').value='';$('profileName').value=($('profileName').value.slice(0,55)||'Profile')+'-copy';notice('Choose a name and Save to library to keep this duplicate.');});
listen('store','click',async()=>{const name=$('profileName').value.trim();const existing=profiles.find(p=>p.name.toLowerCase()===name.toLowerCase());if(existing&&!confirm(`Replace the saved library profile ${existing.name}?`))return;await api('profile',{name:existing?.name||name,text:text(),expected:existing?.text||null});await library();$('profiles').value=existing?.name||name;notice('Profile saved to the library. Apply changes separately to activate this draft.');});
listen('import','click',()=>$('file').click());
listen('file','change',async()=>{const file=$('file').files[0];$('file').value='';if(!file)return;if(file.size>65536)throw Error('Configuration files must be at most 64 KiB.');const imported=await file.text();await api('validate',{text:imported});if(dirty()&&!confirm('Replace the current draft with this import?'))return;draft=JSON.parse(imported.replace(/^\uFEFF/,''));layer=0;$('profileName').value=file.name.replace(/\.json$/i,'').replace(/[^a-z0-9_-]/gi,'-').slice(0,64)||'Imported';$('profiles').value='';render();notice('JSON validated and imported as a draft. Apply changes when ready.');});
listen('export','click',async()=>{await api('validate',{text:text()});const blob=new Blob([text()],{type:'application/json'}),url=URL.createObjectURL(blob),a=document.createElement('a');a.href=url;a.download=($('profileName').value.replace(/[^a-z0-9_-]/gi,'-')||'Tartarus')+'.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);notice('Draft exported as JSON.');});
const scanCodes={Digit1:2,Digit2:3,Digit3:4,Digit4:5,Digit5:6,Digit6:7,Digit7:8,Digit8:9,Digit9:10,Digit0:11,Minus:12,Equal:13,BracketLeft:26,BracketRight:27,Semicolon:39,Quote:40,Backquote:41,Backslash:43,Comma:51,Period:52,Slash:53,CapsLock:58,F1:59,F2:60,F3:61,F4:62,F5:63,F6:64,F7:65,F8:66,F9:67,F10:68,F11:87,F12:88};
function keyName(code){if(/^Key[A-Z]$/.test(code))return code.slice(3);const named={ControlLeft:'LeftCtrl',ControlRight:'RightCtrl',AltLeft:'LeftAlt',AltRight:'RightAlt',ShiftLeft:'LeftShift',ShiftRight:'RightShift',MetaLeft:'LeftWin',MetaRight:'RightWin',Space:'Space',Tab:'Tab',Enter:'Enter',Escape:'Escape',Backspace:'Backspace',AudioVolumeUp:'VolumeUp',AudioVolumeDown:'VolumeDown',AudioVolumeMute:'VolumeMute',MediaPlayPause:'MediaPlayPause',MediaStop:'MediaStop',MediaTrackNext:'MediaNextTrack',MediaTrackPrevious:'MediaPreviousTrack',ArrowUp:'Up',ArrowDown:'Down',ArrowLeft:'Left',ArrowRight:'Right'};if(named[code])return named[code];if(scanCodes[code])return 'scan:'+scanCodes[code].toString(16).padStart(4,'0');const ext={Home:71,PageUp:73,End:79,PageDown:81,Insert:82,Delete:83};return ext[code]?'scan:E0:'+ext[code].toString(16):null;}
function finishRecord(cancel=false){if(!recording)return;clearTimeout(timer);if(!cancel&&recording.keys.length){const keys=recording.keys;const ordered=[...keys.filter(k=>modifiers.includes(k)),...keys.filter(k=>!modifiers.includes(k))];$('action').value=ordered.length>1?'chord':'key';$('keys').value=ordered.join(' + ');actionFields();notice('Combination recorded. Update assignment, then Apply changes.');}else notice('Recording cancelled.');recording=null;$('record').textContent='Record combination';}
listen('record','click',()=>{if(recording){finishRecord(true);return;}recording={keys:[],held:new Set()};$('record').textContent='Recording… Escape cancels';$('record').focus();notice('Press and release your combination. Recording ends after release or 10 seconds.');timer=setTimeout(()=>finishRecord(true),10000);});
window.addEventListener('keydown',e=>{if(!recording)return;e.preventDefault();e.stopPropagation();if(e.code==='Escape'){finishRecord(true);return;}const key=keyName(e.code);if(!key){finishRecord(true);notice('This key cannot be recorded yet. Enter its scan code manually.',true);return;}recording.held.add(e.code);if(!recording.keys.includes(key))recording.keys.push(key);if(recording.keys.length>8){finishRecord(true);notice('Combinations support up to eight distinct keys.',true);}},true);
window.addEventListener('keyup',e=>{if(!recording)return;e.preventDefault();e.stopPropagation();recording.held.delete(e.code);if(!recording.held.size)finishRecord();},true);
window.addEventListener('blur',()=>finishRecord(true));
window.addEventListener('beforeunload',e=>{if(dirty()){e.preventDefault();e.returnValue='';}});
async function poll(){clearTimeout(pollTimer);if(closed)return;await status();if(!document.hidden&&!closed)pollTimer=setTimeout(poll,1500);}
document.addEventListener('visibilitychange',()=>{clearTimeout(pollTimer);if(!document.hidden)poll();});
(async()=>{try{await load();await library();poll();}catch(e){notice(e.message,true);}})();

listen('exit','click',async()=>{if(dirty()&&!confirm('Exit and discard the unsaved draft?'))return;await api('exit',{});closed=true;stopLive();clearTimeout(pollTimer);savedCanonical=canonical(draft);$('status').textContent='Application closed';for(const b of document.querySelectorAll('button'))b.disabled=true;notice('Application closed. You can close this browser tab.');});
let synapsePreview;
function renderSynapse(){
 const c=synapsePreview.candidates[Number($('synapseCandidate').value)];
 $('synapseSummary').textContent=`${c.converted} assignments converted · ${c.inactive} inactive D-pad entries preserved · ${c.warnings.length} items need attention${c.blocked?' · Loading blocked':''}`;
 $('synapseWarnings').replaceChildren();for(const warning of c.warnings){const li=document.createElement('li');li.textContent=warning;$('synapseWarnings').append(li);}
 $('synapseMappings').replaceChildren();
 const controls=[...Array.from({length:20},(_,i)=>String(i+1).padStart(2,'0')),'thumb','wheel_up','wheel_click','wheel_down','dpad_up','dpad_down','dpad_left','dpad_right'];
 for(const name of controls){const tr=document.createElement('tr');const base=c.config.mappings[name],hs=c.config.layers?.[0]?.mappings[name];const trigger=name==='thumb'&&c.config.thumb;for(const text of [controlName(name),trigger?'Hold for HyperShift':actionLabel(base),trigger?'Hold for HyperShift':actionLabel(hs)]){const td=document.createElement('td');td.textContent=text;tr.append(td);}$('synapseMappings').append(tr);}
 $('synapseLoad').disabled=c.blocked;
}
listen('synapseImport','click',()=>$('synapseFile').click());
listen('synapseFile','change',async()=>{
 const file=$('synapseFile').files[0];$('synapseFile').value='';if(!file)return;
 if(file.size>16384)throw Error('This first Synapse importer supports backups up to 16 KiB, so the original fits inside our JSON profile. Your file has not been changed.');
 notice('Reading Synapse backup…');synapsePreview=await api('synapse/preview',{name:file.name,bytes:Array.from(new Uint8Array(await file.arrayBuffer()))});
 $('synapseDevice').textContent=`${synapsePreview.device} · ${file.name} · ${synapsePreview.preserved_bytes.toLocaleString()} original bytes preserved`;
 $('synapseCandidate').replaceChildren();synapsePreview.candidates.forEach((c,i)=>$('synapseCandidate').append(option(i,c.name+(c.selected_in_source?' — selected in backup':''))));
 const index=synapsePreview.candidates.findIndex(c=>c.selected_in_source);$('synapseCandidate').value=String(Math.max(0,index));
 $('synapseNotes').replaceChildren();for(const note of synapsePreview.notes){const p=document.createElement('p');p.textContent=note;$('synapseNotes').append(p);}
 renderSynapse();$('synapseDialog').showModal();notice('Synapse preview ready. Active mappings have not changed.');
});
listen('synapseCandidate','change',renderSynapse);
listen('synapseClose','click',()=>$('synapseDialog').close());
listen('synapseLoad','click',()=>{const c=synapsePreview.candidates[Number($('synapseCandidate').value)];if(c.blocked)return;if(dirty()&&!confirm('Replace the current draft with the selected imported keymap?'))return;draft=structuredClone(c.config);layer=0;$('profiles').value='';$('profileName').value=c.name.replace(/[^a-z0-9_-]+/gi,'-').slice(0,64)||'Synapse-import';$('synapseDialog').close();render();notice(`Loaded ${c.name} as a draft. Original backup is preserved. Review and Apply changes when ready.`);});

// Live input is opt-in. Authentication stays in a request header, never a URL.
let liveController, liveRetry, liveFrame, livePresses={}, liveGeneration=false;
const flashTimers=new Map();
function clearLive(){
 for(const timer of flashTimers.values())clearTimeout(timer);flashTimers.clear();
 document.querySelectorAll('[data-control]').forEach(b=>b.classList.remove('physical-held','physical-flash'));
 livePresses={};liveFrame=null;liveGeneration=false;$('liveLast').textContent='No Tartarus input observed';
}
function followFrame(){
 if(!liveFrame||!$('liveEnabled').checked)return;
 const editing=document.activeElement?.matches('#keys,#layerName,#profileName,#action,#thumbMode,#thumbValue');
 if(liveFrame.running&&$('followLayer').checked&&!dirty()&&!editing&&liveFrame.layer<layerNames().length&&layer!==liveFrame.layer){layer=liveFrame.layer;render();}
 $('liveHint').textContent=!liveFrame.running?'Start remapping to observe the Tartarus. Other keyboards are excluded.':$('followLayer').checked&&(dirty()||editing)?'Live input active · layer following paused while you edit.':'Live input active · physical controls, independent of which editor field has focus.';
}
function paintLive(frame){
 if(!frame.running){clearLive();liveFrame=frame;followFrame();return;}
 if(!liveGeneration){livePresses={...frame.presses};liveGeneration=true;}
 liveFrame=frame;const held=new Set(frame.held);
 document.querySelectorAll('[data-control]').forEach(b=>{
  const name=b.dataset.control;b.classList.toggle('physical-held',held.has(name));
  if((frame.presses[name]||0)>(livePresses[name]||0)){
   b.classList.add('physical-flash');clearTimeout(flashTimers.get(name));flashTimers.set(name,setTimeout(()=>{b.classList.remove('physical-flash');flashTimers.delete(name);},200));
  }
 });
 livePresses={...frame.presses};
 $('liveLast').textContent=frame.last_control?`${controlName(frame.last_control)} → ${frame.last_action||'Native'}`:'No Tartarus input observed';
 $('activeLayer').textContent=`Active: ${frame.layer_names[frame.layer]||frame.layer}`;
 followFrame();
}
function stopLive(){clearTimeout(liveRetry);liveController?.abort();liveController=null;clearLive();}
async function connectLive(){
 stopLive();if(closed||document.hidden||!$('liveEnabled').checked)return;
 const controller=new AbortController();liveController=controller;$('liveHint').textContent='Connecting live input…';
 try{
  const response=await fetch('/api/live',{headers:{'X-Tartarus-Token':token},signal:controller.signal});
  if(!response.ok){let message='Live view unavailable';try{message=(await response.json()).error||message;}catch{}throw Error(message);}
  const reader=response.body.getReader(),decoder=new TextDecoder();let pending='';
  while(!controller.signal.aborted){
   const {value,done}=await reader.read();if(done)throw Error('Live connection closed');pending+=decoder.decode(value,{stream:true});
   if(pending.length>65536)throw Error('Invalid live message');
   let end;while((end=pending.indexOf('\n\n'))>=0){const chunk=pending.slice(0,end);pending=pending.slice(end+2);if(chunk.startsWith('data: ')){const message=JSON.parse(chunk.slice(6));paintLive(message.frame);}}
  }
 }catch(e){if(!controller.signal.aborted){clearLive();$('liveHint').textContent=`${e.message}. Retrying…`;liveRetry=setTimeout(connectLive,3000);}}
}
listen('liveEnabled','change',()=>{if($('liveEnabled').checked)connectLive();else{stopLive();$('liveHint').textContent='Live view off · no input stream open.';}});
listen('followLayer','change',followFrame);
document.addEventListener('visibilitychange',()=>{if(document.hidden)stopLive();else if($('liveEnabled').checked)connectLive();});
window.addEventListener('pagehide',stopLive);

function appearanceReport(){
 const css=getComputedStyle(document.body);
 $('appearanceReport').textContent=`Build: Live view + bundled font\nFont: ${css.fontFamily}\nBundled font loaded: ${document.fonts.check('14px "Tartarus UI"')}\nBackground: ${css.backgroundColor}\nPage color scheme: ${getComputedStyle(document.documentElement).colorScheme}\nBrowser forced colors: ${matchMedia('(forced-colors: active)').matches}`;
}
document.fonts.ready.then(appearanceReport);
listen('appearanceDetails','toggle',appearanceReport);

let launchSettings;
function paintLaunchSettings(s){
 launchSettings=s;
 $('showTray').checked=s.show_tray;$('autostart').checked=s.autostart;$('autoRemap').checked=s.auto_remap;$('openBrowser').checked=s.open_browser;
 $('startupStatus').textContent=s.autostart?(s.registration_matches?'Registered to start at Windows sign-in.':'A startup entry points to another location. Disable it before registering this copy.'):'Windows autostart is off.';
}
async function loadLaunchSettings(){paintLaunchSettings(await api('settings'));}
listen('reloadStartup','click',loadLaunchSettings);
listen('saveStartup','click',async()=>{
 if(!launchSettings)throw Error('Reload launch settings first.');
 $('saveStartup').disabled=true;
 try{paintLaunchSettings(await api('settings',{expected:launchSettings.expected,autostart:$('autostart').checked,auto_remap:$('autoRemap').checked,open_browser:$('openBrowser').checked,show_tray:$('showTray').checked}));notice('App settings saved. Tray visibility updated; current remapping is unchanged.');}
 finally{$('saveStartup').disabled=false;}
});
loadLaunchSettings().catch(e=>{$('startupStatus').textContent=e.message;});

let rgbSettings;
function rgbFields(){
 const managed=$('rgbManaged').checked,effect=$('rgbEffect').value;
 for(const id of ['rgbEffect','rgbColor','rgbBrightness','rgbSpeed','rgbDirection'])$(id).disabled=!managed;
 $('rgbColorLabel').hidden=!['static','breathing','reactive'].includes(effect);
 $('rgbSpeedLabel').hidden=effect!=='reactive';$('rgbDirectionLabel').hidden=effect!=='wave';
 $('rgbBrightnessValue').textContent=$('rgbBrightness').value+'%';
}
function paintRgbStatus(s){if(!s)return;$('rgbStatus').textContent=s.status+(s.error?' · '+s.error:'');}
async function loadRgb(){
 const result=await api('lighting');rgbSettings=result;
 const s=result.settings;
 $('rgbManaged').checked=s.managed;$('rgbEffect').value=s.effect;$('rgbColor').value='#'+s.color.map(v=>v.toString(16).padStart(2,'0')).join('');
 $('rgbBrightness').value=String(s.brightness);$('rgbSpeed').value=String(s.speed);$('rgbDirection').value=String(s.direction);rgbFields();paintRgbStatus(result);
}
for(const id of ['rgbManaged','rgbEffect'])listen(id,'change',rgbFields);
listen('rgbBrightness','input',rgbFields);
listen('rgbReload','click',loadRgb);
listen('rgbApply','click',async()=>{
 if(!rgbSettings)throw Error('Reload lighting settings first.');
 const color=$('rgbColor').value;
 const settings={schema_version:1,managed:$('rgbManaged').checked,effect:$('rgbEffect').value,color:[1,3,5].map(i=>parseInt(color.slice(i,i+2),16)),brightness:Number($('rgbBrightness').value),speed:Number($('rgbSpeed').value),direction:Number($('rgbDirection').value)};
 $('rgbApply').disabled=true;
 try{rgbSettings=await api('lighting',{expected:rgbSettings.expected,settings});paintRgbStatus(rgbSettings);notice('Lighting saved. Check its status for the device response.');}
 finally{$('rgbApply').disabled=false;}
});
loadRgb().catch(e=>{$('rgbStatus').textContent=e.message;});
function renderMacro(){
 const list=$('macroSteps');list.replaceChildren();
 const locked=selected==='thumb'&&!!draft.thumb;
 macroDraft.forEach((step,i)=>{
  const row=document.createElement('li');row.className='macro-step';
  const kind=document.createElement('select');kind.setAttribute('aria-label','Step '+(i+1)+' action');
  [['tap','Keyboard / combination'],['delay','Pause'],['mouse_button','Mouse button'],['mouse_wheel','Mouse wheel']].forEach(([v,t])=>kind.append(option(v,t)));
  kind.value=step.type;kind.disabled=locked;row.append(kind);
  kind.addEventListener('change',()=>{macroDraft[i]=kind.value==='delay'?{type:'delay',ms:100}:kind.value==='tap'?{type:'tap',keys:['A'],hold_ms:40}:kind.value==='mouse_button'?{type:'mouse_button',button:'left',hold_ms:40}:{type:'mouse_wheel',direction:'up'};renderMacro();});
  if(step.type==='tap'){
   const label=document.createElement('label');label.textContent='Keys';
   const field=document.createElement('input');field.value=step.keys.join(' + ');field.placeholder='LeftCtrl + C';field.disabled=locked;
   field.setAttribute('aria-label','Step '+(i+1)+' keys');
   field.addEventListener('input',()=>{step.keys=field.value.split('+').map(x=>x.trim()).filter(Boolean);});label.append(field);row.append(label);
  }
  if(['mouse_button','mouse_wheel'].includes(step.type)){
   const field=document.createElement('select');field.setAttribute('aria-label','Step '+(i+1)+' mouse action');field.disabled=locked;
   const choices=step.type==='mouse_button'?['left','right','middle','back','forward']:['up','down','left','right'];
   choices.forEach(x=>field.append(option(x,x)));const prop=step.type==='mouse_button'?'button':'direction';field.value=step[prop];field.addEventListener('change',()=>{step[prop]=field.value;});row.append(field);
  }
  if(step.type!=='mouse_wheel'){
   const label=document.createElement('label');label.textContent=step.type==='delay'?'Pause (ms)':'Hold (ms)';
   const field=document.createElement('input');field.type='number';field.min='1';field.max='10000';field.step='1';field.disabled=locked;
   field.setAttribute('aria-label','Step '+(i+1)+' duration');
   const prop=step.type==='delay'?'ms':'hold_ms';field.value=step[prop]??40;field.addEventListener('input',()=>{step[prop]=Number(field.value);macroSummary();});label.append(field);row.append(label);
  }
  const actions=document.createElement('div');actions.className='macro-row-actions';
  for(const [label,move] of [['Move left',-1],['Move right',1],['Remove',0]]){
   const b=document.createElement('button');b.type='button';b.className='quiet';b.textContent=move<0?'←':move>0?'→':'Remove';b.title=label;b.setAttribute('aria-label',label+' step '+(i+1));b.disabled=locked||(move===-1&&i===0)||(move===1&&i===macroDraft.length-1);
   b.addEventListener('click',()=>{if(move){[macroDraft[i],macroDraft[i+move]]=[macroDraft[i+move],macroDraft[i]];}else macroDraft.splice(i,1);renderMacro();});actions.append(b);
  }
  row.append(actions);list.append(row);
 });
 $('macroAdd').disabled=locked||macroDraft.length>=128;macroSummary();
}
function macroSummary(){
 const ms=macroDraft.reduce((sum,s)=>sum+(s.type==='delay'?s.ms:s.type==='mouse_wheel'?1:(s.hold_ms??40)),0);
 $('macroSummary').textContent=macroDraft.length+' / 128 steps · '+ms+' ms planned · maximum 60 seconds. Timing is best effort.';
}
listen('macroAdd','click',()=>{if(macroDraft.length<128){macroDraft.push({type:'tap',keys:['A'],hold_ms:40});renderMacro();}});
