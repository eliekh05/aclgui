#!/usr/bin/env node
import { run } from 'bin-shim';
run({ scope: 'aclgui', binaryName: 'aclgui', from: import.meta.url });
