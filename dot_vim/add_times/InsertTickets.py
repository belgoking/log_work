import os
import os.path
import vim

def splitTicketNumberFromDescription(line):
	try:
		p=line.index(' ')
		return line[:p]
	except ValueError:
		return line

def getNumberedTicketsWithDescriptions():
	ticketsPath=os.path.dirname(vim.current.buffer.name)+"/.tickets.cfg"
	if not os.path.exists(ticketsPath): ticketsPath=os.environ['HOME']+"/.tickets.cfg"
	if not os.path.exists(ticketsPath): return None
	return map(lambda x: x.strip(), open(ticketsPath).readlines())

def readVimInput():
	vim.command('call inputsave()')
	vim.command('let user_input = input("")')
	vim.command('call inputrestore()')
	return vim.eval('user_input')

def readSelection():
	entries=getNumberedTicketsWithDescriptions()
	if not entries: return [""]
	entries=map(lambda x: x, enumerate(entries))
	print('\n'.join(map(lambda posNline: str(posNline[0])+'. '+posNline[1], entries)) + '\n\nPlease select the entries separated by commas: ')
	selections=readVimInput()
	choicesMap=dict(entries)
	output=[]
	for selection in selections.split(','):
		try:
			output.append(splitTicketNumberFromDescription(choicesMap[int(selection)]))
		except (KeyError,ValueError):
			if selection: output.append(selection)
	return output

def insertTicketNames():
	output='|'.join(readSelection())
	vim.current.line+=output
	cursorY=vim.current.window.cursor[0]
	vim.current.window.cursor = (cursorY, len(vim.current.line)-1)
