import './lib/theme/tokens.css'
import './lib/theme/base.css'
import { mount } from 'svelte'
import App from './App.svelte'

const target = document.getElementById('app')
if (!target) throw new Error('missing #app')

mount(App, { target })
