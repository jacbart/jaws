package tui

import (
	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/jacbart/jaws/utils/style"
)

type (
	errMsg error
)

var (
	inputStyle = lipgloss.NewStyle().Foreground(style.Blue)
)

type inputModel struct {
	inputs  []textinput.Model
	focused int
	err     error
	vars    []ModelVars
}

type ModelVars struct {
	Description string
	Placeholder string
	Width       int
}

func initialInputModel(vars []ModelVars) inputModel {
	var inputs []textinput.Model = make([]textinput.Model, len(vars))
	for i, v := range vars {
		inputs[i] = textinput.New()
		if v.Placeholder != "" {
			inputs[i].Placeholder = v.Placeholder
		}

		if v.Width != -1 {
			inputs[i].CharLimit = v.Width
			inputs[i].Width = v.Width
		} else {
			inputs[i].CharLimit = 60
			inputs[i].Width = 70
		}
		inputs[i].Prompt = ""
	}
	inputs[0].Focus()

	return inputModel{
		inputs:  inputs,
		focused: 0,
		err:     nil,
		vars:    vars,
	}
}

func (m inputModel) Init() tea.Cmd {
	return textinput.Blink
}

func (m inputModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd = make([]tea.Cmd, len(m.inputs))

	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.Type {
		case tea.KeyEnter:
			if m.inputs[m.focused].Value() == "" {
				m.inputs[m.focused].SetValue(m.inputs[m.focused].Placeholder)
			}
			if m.focused == len(m.inputs)-1 {
				return m, tea.Quit
			}
			m.nextInput()
		case tea.KeyCtrlC, tea.KeyEsc:
			return m, tea.Quit
		case tea.KeyShiftTab, tea.KeyCtrlP:
			m.prevInput()
		case tea.KeyTab, tea.KeyCtrlN:
			m.nextInput()
		}
		for i := range m.inputs {
			m.inputs[i].Blur()
		}
		m.inputs[m.focused].Focus()

	case errMsg:
		m.err = msg
		return m, nil
	}

	for i := range m.inputs {
		m.inputs[i], cmds[i] = m.inputs[i].Update(msg)
	}
	return m, tea.Batch(cmds...)
}

func (m inputModel) View() string {
	var content string
	for i, in := range m.inputs {
		content = content + inputStyle.Render(m.vars[i].Description) + "\n" + in.View() + "\n"
	}
	return content
}

// nextInput focuses the next input field
func (m *inputModel) nextInput() {
	m.focused = (m.focused + 1) % len(m.inputs)
}

// prevInput focuses the previous input field
func (m *inputModel) prevInput() {
	m.focused--
	// Wrap around
	if m.focused < 0 {
		m.focused = len(m.inputs) - 1
	}
}

func InputTUI(vars []ModelVars) ([]string, error) {
	m := initialInputModel(vars)
	p := tea.NewProgram(&m)

	if err := p.Start(); err != nil {
		return []string{}, err
	}
	var vals []string
	for _, in := range m.inputs {
		vals = append(vals, in.Value())
	}
	return vals, nil
}
