package style

import "github.com/charmbracelet/lipgloss"

const (
	DarkGray = lipgloss.Color("#767676")
	Blue     = lipgloss.Color("#0088ce")
	DarkBlue = lipgloss.Color("#002a4e")
	White    = lipgloss.Color("#fefefe")
	Grey     = lipgloss.Color("#768d99")
	Green    = lipgloss.Color("#11bc58")
	Orange   = lipgloss.Color("#e08745")
	Red      = lipgloss.Color("#ff9563")
)

var (
	SuccessString  = lipgloss.NewStyle().Foreground(Green).Render
	FailureString  = lipgloss.NewStyle().Foreground(Red).Render
	WarningString  = lipgloss.NewStyle().Foreground(Orange).Render
	InfoString     = lipgloss.NewStyle().Foreground(Blue).Render
	InfoHintString = lipgloss.NewStyle().Foreground(DarkBlue).Faint(true).Render
	ChangedString  = lipgloss.NewStyle().Foreground(Orange).Faint(true).Render

	BlueStyle     = lipgloss.NewStyle().Foreground(Blue)
	DarkBlueStyle = lipgloss.NewStyle().Foreground(DarkBlue)
	WhiteStyle    = lipgloss.NewStyle().Foreground(White)
	GreyStyle     = lipgloss.NewStyle().Foreground(Grey)
	GreenStyle    = lipgloss.NewStyle().Foreground(Green)
)
