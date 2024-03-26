package style

import "github.com/charmbracelet/lipgloss"

const (
	DarkGray     = lipgloss.Color("#767676")
	JrnyBlue     = lipgloss.Color("#0088ce")
	JrnyDarkBlue = lipgloss.Color("#002a4e")
	White        = lipgloss.Color("#fefefe")
	Grey         = lipgloss.Color("#768d99")
	Green        = lipgloss.Color("#11bc58")
	Orange       = lipgloss.Color("#e08745")
	Red          = lipgloss.Color("#ff9563")
)

var (
	SuccessString  = lipgloss.NewStyle().Foreground(Green).Render
	FailureString  = lipgloss.NewStyle().Foreground(Red).Render
	WarningString  = lipgloss.NewStyle().Foreground(Orange).Render
	InfoString     = lipgloss.NewStyle().Foreground(JrnyBlue).Render
	InfoHintString = lipgloss.NewStyle().Foreground(JrnyDarkBlue).Faint(true).Render
	ChangedString  = lipgloss.NewStyle().Foreground(Orange).Faint(true).Render

	JrnyBlueStyle     = lipgloss.NewStyle().Foreground(JrnyBlue)
	JrnyDarkBlueStyle = lipgloss.NewStyle().Foreground(JrnyDarkBlue)
	WhiteStyle        = lipgloss.NewStyle().Foreground(White)
	GreyStyle         = lipgloss.NewStyle().Foreground(Grey)
	GreenStyle        = lipgloss.NewStyle().Foreground(Green)
)
