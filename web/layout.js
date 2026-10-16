'use strict';
// Move existing controls into focused views; their listeners and draft values stay intact.
(()=>{
 const main=document.querySelector('main'), workspace=document.querySelector('.workspace');
 const profiles=main.querySelector('aside:not(.inspector)'), inspector=main.querySelector('.inspector');
 const head=workspace.querySelector('.section-head'), notice=document.getElementById('notice');
 const layerBar=workspace.querySelector('.layer-bar'), thumb=workspace.querySelector('.thumb-settings');
 const lighting=workspace.querySelector('.lighting-settings'), live=workspace.querySelector('.live-bar');
 const boardArea=workspace.querySelector('.board-area'), board=workspace.querySelector('.board'), note=workspace.querySelector('.board-note');
 const startup=profiles.querySelector('.startup-settings'), appearance=document.getElementById('appearanceDetails');
 const hrs=[...profiles.querySelectorAll('hr')], connection=document.createElement('section');
 connection.className='settings-card';connection.setAttribute('aria-label','Connection status');
 let node=hrs[0].nextSibling;
 while(node&&node!==hrs[1]){const next=node.nextSibling;connection.append(node);node=next;}
 hrs.forEach(n=>n.remove());startup.open=true;startup.querySelector('summary').textContent='Startup & background';
 const settings=document.createElement('div');settings.className='settings-grid';settings.append(startup,connection,appearance);
 const nav=document.createElement('nav');nav.className='view-tabs';nav.setAttribute('role','tablist');nav.setAttribute('aria-label','Configuration sections');
 const views=document.createElement('div');views.className='view-container';
 const keypad=document.createElement('section');keypad.className='keypad-layout';
 const keyMain=document.createElement('div');keyMain.className='keypad-main';
 const boardViewport=document.createElement('div');boardViewport.className='board-viewport';boardArea.replaceWith(boardViewport);boardViewport.append(boardArea);
 const liveDetails=document.createElement('details');liveDetails.className='live-details';
 const liveSummary=document.createElement('summary');liveSummary.textContent='Live input & visual debugging';liveDetails.append(liveSummary,live);
 const calibration=document.createElement('details');calibration.className='calibration';
 const calibrationSummary=document.createElement('summary');calibrationSummary.textContent='About D-pad directions';calibration.append(calibrationSummary,note);
 keyMain.append(layerBar,boardViewport,liveDetails,calibration);keypad.append(keyMain,inspector);workspace.append(keypad);
 const layers=document.createElement('div');layers.className='settings-page';
 const layerTools=document.createElement('div');layerTools.className='layer-tools';
 layerTools.append(document.getElementById('addLayer'),layerBar.querySelector('.layer-name'));
 const layerIntro=document.createElement('p');layerIntro.className='hint';layerIntro.textContent='Choose a layer above to edit its name. Assign its keys in Keypad. Unassigned controls retain native input.';
 layers.append(layerTools,layerIntro,thumb);
 profiles.classList.add('profiles-page');lighting.classList.add('settings-page');
 const profileColumns=document.createElement('div');profileColumns.className='profile-columns';
 const profileForm=document.createElement('section'),profileTransfer=document.createElement('section');
 profileForm.className=profileTransfer.className='settings-card';
 const transferTitle=document.createElement('h3');transferTitle.textContent='Import & export';profileTransfer.append(transferTitle);
 profileForm.append(document.getElementById('profiles').closest('label'),document.getElementById('new').parentElement,document.getElementById('profileName').closest('label'),document.getElementById('store'));
 profileTransfer.append(document.getElementById('import').parentElement,document.getElementById('file'),document.getElementById('synapseImport'),document.getElementById('synapseFile'),profiles.querySelector('p.hint'));
 profileColumns.append(profileForm,profileTransfer);profiles.append(profileColumns);
 const back=document.createElement('button');back.type='button';back.className='quiet inspector-back';back.textContent='← Back to keypad';inspector.prepend(back);
 const sequenceButton=document.createElement('button');sequenceButton.type='button';sequenceButton.className='quiet sequence-open';sequenceButton.textContent='Open wide sequence editor';sequenceButton.hidden=true;document.getElementById('action').parentElement.after(sequenceButton);
 const steps=document.getElementById('macroSteps'), add=document.getElementById('macroAdd'), track=document.createElement('div');
 track.className='macro-track';steps.before(track);track.append(steps,add);add.textContent='+ Add step';
 const tabs=[['keypad','Keypad',keypad],['layers','Layers & thumb',layers],['lighting','Lighting',lighting],['profiles','Profiles',profiles],['settings','App settings',settings]];
 let current='keypad', sequenceOpen=true;
 const panelMap=new Map(),buttons=[];
 const choose=id=>{
  current=id;
  head.lastElementChild.hidden=id==='lighting'||id==='settings';
  for(const [key,panel] of panelMap){panel.hidden=key!==id;const b=document.getElementById('tab-'+key);b.setAttribute('aria-selected',String(key===id));b.tabIndex=key===id?0:-1;}
  if(id==='keypad')keyMain.prepend(layerBar);else if(id==='layers')layers.prepend(layerBar);
  head.querySelector('h1').textContent={keypad:'Customize your keypad',layers:'Layers & thumb',lighting:'Lighting',profiles:'Your profiles',settings:'App settings'}[id];
  head.querySelector('.muted').textContent={keypad:'Select a control to change what it does.',layers:'Set up layers and how the thumb switches between them.',lighting:'Set the backlighting for your device.',profiles:'Create, save and transfer your mappings.',settings:'Startup, connection and appearance details.'}[id];
  requestAnimationFrame(fitBoard);
 };
 for(const [id,label,content] of tabs){
  const b=document.createElement('button');b.type='button';b.id='tab-'+id;b.textContent=label;b.setAttribute('role','tab');b.setAttribute('aria-controls','view-'+id);
  const panel=document.createElement('section');panel.id='view-'+id;panel.className='view-panel';panel.setAttribute('role','tabpanel');panel.setAttribute('aria-labelledby',b.id);panel.tabIndex=0;panel.append(content);
  panelMap.set(id,panel);views.append(panel);nav.append(b);buttons.push(b);b.addEventListener('click',()=>choose(id));
  b.addEventListener('keydown',e=>{let index=buttons.indexOf(b);if(e.key==='ArrowRight')index=(index+1)%tabs.length;else if(e.key==='ArrowLeft')index=(index+tabs.length-1)%tabs.length;else if(e.key==='Home')index=0;else if(e.key==='End')index=tabs.length-1;else return;e.preventDefault();buttons[index].click();buttons[index].focus();});
 }
 main.replaceChildren(head,nav,notice,views);main.className='editor-shell';document.body.classList.add('app-shell');
 function fitBoard(){
  if(current!=='keypad'||!boardViewport.clientWidth||!boardViewport.clientHeight)return;
  const scale=Math.min(1,boardViewport.clientWidth/board.offsetWidth,boardViewport.clientHeight/board.offsetHeight);
  board.style.transform='scale('+scale+')';boardArea.style.width=board.offsetWidth*scale+'px';boardArea.style.height=board.offsetHeight*scale+'px';
 }
 function syncSequence(){
  const macro=!document.getElementById('macroEditor').hidden;
  sequenceButton.hidden=!macro;
  keypad.classList.toggle('sequence-mode',macro&&sequenceOpen);
  if(macro&&sequenceOpen)keypad.classList.add('show-assignment');
  requestAnimationFrame(fitBoard);
 }
 const pickerLabel=document.createElement('label');pickerLabel.className='compact-picker';pickerLabel.textContent='Choose control';
 const picker=document.createElement('select');picker.id='controlPicker';
 keypad.querySelectorAll('[data-control]').forEach(b=>{const o=document.createElement('option');o.value=b.dataset.control;o.textContent=controlName(b.dataset.control);picker.append(o);b.addEventListener('click',()=>{picker.value=b.dataset.control;});});
 pickerLabel.append(picker);layerBar.append(pickerLabel);picker.addEventListener('change',()=>{keypad.querySelector('[data-control="'+picker.value+'"]').click();});
 const openAssignment=()=>{keypad.classList.add('show-assignment');sequenceOpen=true;syncSequence();if(matchMedia('(max-width: 760px)').matches)inspector.querySelector('#action').focus();};
 keypad.querySelectorAll('[data-control]').forEach(b=>b.addEventListener('click',openAssignment));
 back.addEventListener('click',()=>{keypad.classList.remove('show-assignment','sequence-mode');sequenceOpen=false;requestAnimationFrame(fitBoard);const selected=keypad.querySelector('[data-control][aria-pressed="true"]');selected?.focus();});
 sequenceButton.addEventListener('click',()=>{sequenceOpen=true;keypad.classList.add('show-assignment');syncSequence();});
 document.getElementById('action').addEventListener('change',()=>{sequenceOpen=true;syncSequence();});
 new MutationObserver(syncSequence).observe(document.getElementById('macroEditor'),{attributes:true,attributeFilter:['hidden']});
 new ResizeObserver(fitBoard).observe(boardViewport);
 add.addEventListener('click',()=>{const step=steps.lastElementChild;step?.querySelector('select')?.focus({preventScroll:true});step?.scrollIntoView({block:'nearest',inline:'end',behavior:'smooth'});});
 document.fonts.ready.then(fitBoard);
 choose('keypad');syncSequence();
})();
